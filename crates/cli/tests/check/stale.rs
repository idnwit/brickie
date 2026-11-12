//! Check tests: staleness.

use super::support::*;
use serde_json::json;
use std::path::Path;
use std::process::Command;

#[test]
fn stale_files() {
    let w = world(|_| {});
    let cfg = w.save(&with_bricks(""));
    let o = run(&cfg, &["--format", "json"]);
    assert!(codes(&o, "stale").is_empty());
    // changed, deleted, added
    w.write("ws/crates/eng/src/x.rs", "// edited\n");
    std::fs::remove_file(w.ws().join("crates/adp/src/y.rs")).unwrap();
    w.write("ws/crates/app/src/new.rs", "// new\n");
    w.write("ws/elsewhere/ignored.rs", "// not in a crate dir\n");
    let o = run(&cfg, &["--format", "json"]);
    let msgs: Vec<&str> = codes(&o, "stale/file")
        .iter()
        .map(|d| d["message"].as_str().unwrap())
        .collect();
    assert_eq!(msgs.len(), 3, "{msgs:?}");
    assert!(
        msgs.iter()
            .any(|m| m.contains("crates/eng/src/x.rs") && m.contains("changed"))
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("crates/adp/src/y.rs") && m.contains("deleted"))
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("crates/app/src/new.rs") && m.contains("added"))
    );
    assert_eq!(o.code, 0, "stale findings are warnings");
}

#[test]
fn uncompiled_file_is_not_added() {
    let w = world(|c| {
        let text = "// cfg-gated away\n";
        c["files"]["crates/eng/src/gated.rs"] = json!({
            "sha256": sha(text), "compiled": false, "size": text.len(), "lines": 1,
            "scope": "prod", "module": null, "functions": [], "edges": [], "items": [],
        });
    });
    w.write("ws/crates/eng/src/gated.rs", "// cfg-gated away\n");
    let cfg = w.save(&with_bricks(""));
    assert!(codes(&run(&cfg, &["--format", "json"]), "stale").is_empty());
    // Changing it is still noticed.
    w.write("ws/crates/eng/src/gated.rs", "// edited\n");
    let o = run(&cfg, &["--format", "json"]);
    let d = codes(&o, "stale/file");
    assert_eq!(d.len(), 1);
    assert!(d[0]["message"].as_str().unwrap().contains("changed"));
}

#[test]
fn config_edits_never_make_the_cache_stale() {
    let w = world(|_| {});
    let moved = BRICKS.replace("members = [\"app\"]", "members = [\"app\", \"misc\"]");
    let o = check(&w, &moved);
    assert!(codes(&o, "stale").is_empty());
    // The new mapping applies at once: `misc::q` is no longer unmapped.
    assert!(codes(&o, "bricks/unmapped").is_empty());
}

fn git(dir: &Path, args: &[&str]) {
    let s = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        s.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&s.stderr)
    );
}

#[test]
fn stale_uses_git_candidates() {
    let w = world(|_| {});
    let ws = w.ws();
    git(&ws, &["init", "-q"]);
    git(&ws, &["add", "-A"]);
    git(&ws, &["commit", "-q", "-m", "x"]);
    let head = Command::new("git")
        .arg("-C")
        .arg(&ws)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    let w = World {
        cache: {
            let mut c = w.cache.clone();
            c["git"] = json!({"commit": head, "dirty": false});
            c
        },
        ..w
    };
    let cfg = w.save(&with_bricks(""));
    assert!(codes(&run(&cfg, &["--format", "json"]), "stale").is_empty());
    // A touched file with identical content is not stale (sha confirms).
    let x = std::fs::read_to_string(ws.join("crates/eng/src/x.rs")).unwrap();
    w.write("ws/crates/eng/src/x.rs", &x);
    assert!(codes(&run(&cfg, &["--format", "json"]), "stale").is_empty());
    // Modified and untracked files are found; an unrelated untracked file is not.
    w.write("ws/crates/eng/src/x.rs", "// edited\n");
    w.write("ws/crates/app/src/new.rs", "// new\n");
    w.write("ws/notes.txt", "x");
    let o = run(&cfg, &["--format", "json"]);
    assert_eq!(codes(&o, "stale/file").len(), 2, "{}", o.stdout);
    // A cached commit that is gone falls back to hashing everything.
    let mut c = w.cache.clone();
    c["git"] = json!({"commit": "0".repeat(40), "dirty": false});
    let w2 = World { cache: c, ..w };
    let cfg = w2.save(&with_bricks(""));
    let o = run(&cfg, &["--format", "json"]);
    assert_eq!(codes(&o, "stale/file").len(), 2, "{}", o.stdout);
}
