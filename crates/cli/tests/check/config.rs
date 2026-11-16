//! Check tests: include and config errors.

use super::support::*;
use serde_json::json;

#[test]
fn include_merges_relative_to_including_file() {
    let w = world(|_| {});
    w.write(
        "presets/layers.toml",
        "include = [\"more/extra.toml\"]\n[[rule]]\nfrom = \"tag:core\"\nto = \"tag:backend\"\naction = \"deny\"\n",
    );
    w.write(
        "presets/more/extra.toml",
        "[[complexity]]\nbricks = \"tag:core\"\ncognitive = 15\n",
    );
    let o = check(
        &w,
        &format!(
            "include = [\"presets/layers.toml\"]\n{BRICKS}\n[[rule]]\nfrom = \"*\"\nto = \"*\"\naction = \"allow\"\n"
        ),
    );
    assert_eq!(codes(&o, "deps/deny").len(), 1);
    assert_eq!(codes(&o, "complexity").len(), 1);
    assert!(
        codes(&o, "deps/deny")[0]["rule"]["loc"]
            .as_str()
            .unwrap()
            .ends_with("presets/layers.toml:2")
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
            with_bricks("[[rule]]\nfrom = \"tag:nope\"\nto = \"*\"\naction = \"deny\"\n"),
            "unknown tag 'nope'",
        ),
        (
            with_bricks("[[rule]]\nfrom = \"*\"\nto = \"ghost\"\naction = \"deny\"\n"),
            "unknown brick 'ghost'",
        ),
        (
            with_bricks("[[complexity]]\nbricks = \"ghost\"\ncognitive = 1\n"),
            "unknown brick 'ghost'",
        ),
        (
            with_bricks("[[rule]]\nfrom = \"*\"\nto = \"*\"\naction = \"deny\"\nbogus = 1\n"),
            "bogus",
        ),
        (with_bricks("[deps]\ndefault = \"allow\"\n"), "deps"),
        ("bogus = 1\n".to_string(), "bogus"),
        (
            with_bricks(
                "[[rule]]\nfrom = \"*\"\nto = \"*\"\nkinds = [\"calls\"]\naction = \"deny\"\n",
            ),
            "unknown kind 'calls'",
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
