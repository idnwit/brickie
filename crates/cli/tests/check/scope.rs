//! Check tests: `--brick` ownership, the footers, the exit code, and `--skip`.

use super::nested::{ROOT, WEB, tree};
use super::support::*;

fn nested(extra: &[&str]) -> Out {
    let w = tree();
    w.write("ws/crates/web/brickie.toml", WEB);
    let config = w.save(ROOT);
    let mut args = vec!["--format", "json"];
    args.extend(extra);
    run(&config, &args)
}

fn shown(o: &Out) -> Vec<(String, String)> {
    let ds = o.json["diagnostics"].as_array().unwrap();
    let mut v: Vec<_> = ds
        .iter()
        .map(|d| {
            (
                d["code"].as_str().unwrap().to_string(),
                d["message"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    v.sort();
    v
}

#[test]
fn brick_shows_owned_findings_and_counts_the_rest() {
    let all = nested(&[]);
    assert_eq!(all.code, 1, "{}", all.stderr);
    let total = all.json["summary"]["errors"].as_u64().unwrap();

    // `deps/private` is owned by its source (root/core), not the hidden child.
    let o = nested(&["--brick", "core"]);
    let codes_shown: Vec<_> = shown(&o).into_iter().map(|(c, _)| c).collect();
    assert!(
        codes_shown.contains(&"deps/private".to_string()),
        "{}",
        o.stdout
    );
    assert!(!codes_shown.iter().any(|c| c.starts_with("complexity")));
    let outside = o.json["outside_scope"]["errors"].as_u64().unwrap();
    assert_eq!(
        o.json["summary"]["errors"].as_u64().unwrap() + outside,
        total
    );
    assert_eq!(o.code, 1);

    // The complexity finding is owned by root/web/ui and `web` includes descendants.
    let o = nested(&["--brick", "web"]);
    assert!(shown(&o).iter().any(|(c, _)| c == "complexity/cognitive"));
    assert_eq!(o.code, 1);

    // No error is owned by web/api: exit 0, the others are counted outside.
    let o = nested(&["--brick", "web/api"]);
    assert_eq!(o.code, 0, "{}", o.stdout);
    assert_eq!(o.json["summary"]["errors"], 0);
    assert_eq!(o.json["outside_scope"]["errors"].as_u64().unwrap(), total);
}

#[test]
fn brick_matches_by_path_and_rejects_unknown() {
    // `ui` is a child of web, not of the root: a bare name does not match.
    let o = nested(&["--brick", "ui"]);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("--brick ui"), "{}", o.stderr);
    assert_eq!(nested(&["--brick", "root/web/ui"]).code, 1);
}

#[test]
fn text_footer_reports_outside_scope() {
    let w = tree();
    w.write("ws/crates/web/brickie.toml", WEB);
    let config = w.save(ROOT);
    let o = run(&config, &["--brick", "web/api", "--color", "never"]);
    assert_eq!(o.code, 0, "{}", o.stdout);
    assert!(o.stdout.contains("outside scope: "), "{}", o.stdout);
    assert!(o.stdout.trim_end().ends_with("warnings"), "{}", o.stdout);
}

#[test]
fn skip_hides_warnings_only() {
    let w = world(|_| {});
    let warn = with_bricks("[[complexity]]\nbricks = \"*\"\ncognitive = 15\nlevel = \"warn\"\n");
    let config = w.save(&warn);
    let o = run(
        &config,
        &["--format", "json", "--skip", "complexity/cognitive"],
    );
    assert!(codes(&o, "complexity").is_empty(), "{}", o.stdout);
    assert_eq!(o.json["skipped"]["warnings"], 2);
    assert_eq!(o.json["skipped"]["codes"][0], "complexity/cognitive");
    let o = run(
        &config,
        &["--skip", "complexity/cognitive", "--color", "never"],
    );
    assert!(
        o.stdout
            .contains("skipped: 2 warnings (complexity/cognitive)"),
        "{}",
        o.stdout
    );

    // The same code at level error is not hidden and still fails the run.
    let err = with_bricks("[[complexity]]\nbricks = \"tag:core\"\ncognitive = 15\n");
    let config = w.save(&err);
    let o = run(
        &config,
        &[
            "--format",
            "json",
            "--skip",
            "complexity/cognitive,bricks/unmapped",
        ],
    );
    assert_eq!(o.code, 1, "{}", o.stdout);
    assert_eq!(codes(&o, "complexity").len(), 1);
    assert!(codes(&o, "bricks/unmapped").is_empty());
    assert_eq!(o.json["skipped"]["warnings"], 1);

    let o = run(&config, &["--skip", "nope/x"]);
    assert_eq!(o.code, 2);
}
