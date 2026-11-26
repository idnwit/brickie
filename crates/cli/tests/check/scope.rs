//! Check tests: `--brick` ownership, the footer and the exit code.

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
}

#[test]
fn group_scope_filter_and_todo_groups() {
    let w = tree();
    let cfg = r#"
[[group]]
name = "svc"
tags = ["domain"]

[[group]]
name = "empty_g"

[[brick]]
name = "auth"
group = "svc"
members = ["web-api"]

[[brick]]
name = "billing"
group = "svc"
members = ["web-ui"]

[[brick]]
name = "bus"
members = ["core"]

[[complexity]]
bricks = "svc"
cognitive = 5
"#;
    let config = w.save(cfg);

    let by_group = run(&config, &["--format", "json", "--brick", "svc"]);
    assert_eq!(by_group.code, 1, "{}", by_group.stderr);
    let codes: Vec<_> = shown(&by_group).into_iter().map(|(c, _)| c).collect();
    assert!(
        codes.contains(&"complexity/cognitive".to_string()),
        "{}",
        by_group.stdout
    );
    assert_eq!(
        by_group.json["todo"]["planned_groups"],
        serde_json::json!(["root/empty_g"]),
        "{}",
        by_group.stdout
    );

    let by_member = run(&config, &["--format", "json", "--brick", "billing"]);
    assert_eq!(by_member.code, 1, "{}", by_member.stderr);
    assert!(
        shown(&by_member)
            .iter()
            .any(|(c, _)| c == "complexity/cognitive"),
        "{}",
        by_member.stdout
    );
}
