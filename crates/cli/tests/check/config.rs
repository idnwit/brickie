//! Check tests: include and config errors.

use super::support::*;
use serde_json::json;

#[test]
fn include_merges_relative_to_including_file() {
    let w = world(|_| {});
    let link =
        |f: &str, t: &str| format!("[[link]]\nfrom = \"{f}\"\nto = \"{t}\"\nreason = \"needed\"\n");
    w.write(
        "presets/layers.toml",
        &format!(
            "include = [\"more/extra.toml\"]\n[[rule]]\nfrom = \"*\"\nto = \"*\"\nreason = \"open\"\n{}{}{}{}",
            link("engine", "port"),
            link("adapter", "port"),
            link("app", "engine"),
            link("app", "adapter"),
        ),
    );
    w.write(
        "presets/more/extra.toml",
        "[[complexity]]\nbricks = \"tag:core\"\ncognitive = 15\n",
    );
    let o = check(
        &w,
        &format!("include = [\"presets/layers.toml\"]\n{BRICKS}"),
    );
    // The included rule and links apply; only the unlinked engine -> adapter edge remains.
    let d = codes(&o, "deps");
    assert_eq!(d.len(), 1, "{}", o.stdout);
    assert_eq!(d[0]["code"], "deps/unlinked");
    assert_eq!(codes(&o, "complexity").len(), 1);
    assert!(
        codes(&o, "complexity")[0]["rule"]["loc"]
            .as_str()
            .unwrap()
            .ends_with("presets/more/extra.toml:1")
    );
}

#[test]
fn config_errors_exit_2() {
    let w = world(|_| {});
    let cases = [
        (
            format!("{BRICKS}[[brick]]\nname = \"port\"\nmembers = []\n"),
            "duplicate brick 'port'",
        ),
        (
            with_bricks("[[rule]]\nfrom = \"tag:nope\"\nto = \"*\"\nreason = \"r\"\n"),
            "unknown tag 'nope'",
        ),
        (
            with_bricks("[[rule]]\nfrom = \"*\"\nto = \"ghost\"\nreason = \"r\"\n"),
            "unknown brick 'ghost'",
        ),
        (
            with_bricks("[[complexity]]\nbricks = \"ghost\"\ncognitive = 1\n"),
            "unknown brick 'ghost'",
        ),
        (
            with_bricks("[[rule]]\nfrom = \"*\"\nto = \"*\"\nreason = \"r\"\nbogus = 1\n"),
            "bogus",
        ),
        (with_bricks("[deps]\ndefault = \"allow\"\n"), "deps"),
        ("bogus = 1\n".to_string(), "bogus"),
        (
            with_bricks(
                "[[rule]]\nfrom = \"*\"\nto = \"*\"\nkinds = [\"calls\"]\nreason = \"r\"\n",
            ),
            "unknown kind 'calls'",
        ),
        (
            with_bricks("[[rule]]\nfrom = \"*\"\nto = \"*\"\n"),
            "reason",
        ),
        (
            with_bricks("[[rule]]\nfrom = \"*\"\nto = \"*\"\nreason = \"r\"\naction = \"allow\"\n"),
            "action",
        ),
        (
            with_bricks("[[link]]\nfrom = \"engine\"\nto = \"engine\"\nreason = \"r\"\n"),
            "config/invalid",
        ),
        (
            with_bricks("[[link]]\nfrom = \"engine\"\nto = \"ghost\"\nreason = \"r\"\n"),
            "unknown brick 'ghost'",
        ),
        ("include = [\"missing.toml\"]\n".to_string(), "missing.toml"),
    ];
    for (cfg, needle) in cases {
        let o = check(&w, &cfg);
        assert_eq!(o.code, 2, "{needle}: {}", o.stderr);
        assert!(o.stderr.contains(needle), "{needle} not in: {}", o.stderr);
    }
    // Include cycle.
    w.write("a.toml", "include = [\"b.toml\"]\n");
    w.write("b.toml", "include = [\"a.toml\"]\n");
    let o = check(&w, "include = [\"a.toml\"]\n");
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("include cycle"), "{}", o.stderr);
    // Missing cache and wrong schema version.
    let cfg = w.save(&with_bricks(""));
    std::fs::remove_file(w.dir.path().join(".brickie/cache.json")).unwrap();
    let o = run(&cfg, &[]);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("brickie measure"));
    let mut old = w.cache.clone();
    old["schema_version"] = json!(1);
    w.write(".brickie/cache.json", &old.to_string());
    let o = run(&cfg, &[]);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("schema"));
}
