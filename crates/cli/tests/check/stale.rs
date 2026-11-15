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
    assert_eq!(o.code, 1, "stale findings are errors");
    assert!(
        codes(&o, "stale/file")
            .iter()
            .all(|d| d["level"] == "error")
    );
    // An accept of a stale id does not turn the gate green.
    let id = codes(&o, "stale/file")[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    w.write(
        "brickie.review.toml",
        &format!("[[accept]]\nid = \"{id}\"\nreason = \"x\"\n"),
    );
    let o = run(&cfg, &["--format", "json"]);
    assert_eq!(o.code, 1, "{}", o.stdout);
    assert_eq!(o.json["summary"]["accepted"], 0);
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

#[test]
fn interface_of_unknown_brick_is_a_usage_error() {
    let w = world(|_| {});
    let cfg = w.save(&with_bricks(""));
    let o = Command::new(BIN)
        .args(["interface", "nope", "--config"])
        .arg(&cfg)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn cache_from_another_location_is_one_stale_measure() {
    let mut w = world(|_| {});
    let canon = w.dir.path().canonicalize().unwrap().display().to_string();
    w.cache["measured"] = json!({"features": "all", "targets": [], "location": canon});
    let cfg = w.save(&with_bricks(""));
    // Same location (compared canonically): no stale finding.
    let o = run(&cfg, &["--format", "json"]);
    assert!(codes(&o, "stale").is_empty(), "{}", o.stdout);
    // Another recorded location: one stale/measure, and no file is judged.
    let w = world(|c| {
        c["measured"] = json!({"features": "all", "targets": [], "location": "/nowhere/else"});
    });
    let cfg = w.save(&with_bricks(""));
    w.write("ws/crates/eng/src/x.rs", "// edited\n");
    let o = run(&cfg, &["--format", "json"]);
    assert_eq!(codes(&o, "stale/measure").len(), 1, "{}", o.stdout);
    assert!(codes(&o, "stale/file").is_empty());
    assert!(
        codes(&o, "stale/measure")[0]["message"]
            .as_str()
            .unwrap()
            .contains("/nowhere/else")
    );
    assert_eq!(o.code, 1);
}

/// `check --measure` must re-measure even when the old facts make the config fail to load
/// (for example `config/unassigned` for a crate that was removed since).
#[test]
fn a_config_error_does_not_hide_stale_facts() {
    let w = world(|_| {});
    let broken =
        with_bricks("[[deps.rule]]\nfrom = \"no-such-brick\"\nto = \"port\"\naction = \"allow\"\n");
    let cfg = w.save(&broken);
    assert_eq!(run(&cfg, &[]).code, 2);
    assert!(!brickie_check::check::needs_measure(&cfg));
    w.write("ws/crates/eng/src/x.rs", "// edited\n");
    assert!(brickie_check::check::needs_measure(&cfg));
}
