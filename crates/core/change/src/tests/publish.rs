//! `publish` runs `check` on the draft: any error, planned brick or stale cache blocks.

use super::*;
use crate::{Blocked, Track};
use serde_json::json;

const PUB_ROOT: &str = r#"[[brick]]
name = "a"
path = "g"

[[brick]]
name = "b"
members = ["pb"]
"#;

const PUB_G: &str = r#"[[brick]]
name = "x"
members = ["pa"]
"#;

/// A measured workspace: crate `pa` in `g/pa` (composite `a`), `pb` in `crates/pb`, and an
/// edge pb -> pa that no rule allows.
fn measured() -> Ws {
    measured_with(true)
}

/// `measured`, with or without the edge no rule allows.
fn measured_with(edge_pb_pa: bool) -> Ws {
    let ws = Ws::new();
    ws.write("brickie.toml", PUB_ROOT);
    ws.write("g/brickie.toml", PUB_G);
    ws.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"g/pa\", \"crates/pb\"]\n",
    );
    let mut crates = Vec::new();
    let mut modules = Vec::new();
    let mut files = serde_json::Map::new();
    for (p, dir, edges) in [
        ("pa", "g/pa", json!([])),
        (
            "pb",
            "crates/pb",
            if edge_pb_pa {
                edge("pb", "pa")
            } else {
                json!([])
            },
        ),
    ] {
        let src = format!("{dir}/src/lib.rs");
        let text = format!("// {p}\n");
        ws.write(
            &format!("{dir}/Cargo.toml"),
            &format!("[package]\nname = \"{p}\"\n"),
        );
        ws.write(&src, &text);
        crates.push(
            json!({"id": p, "package": p, "version": "0", "target": p, "kind": "lib",
                           "root": src, "dir": dir}),
        );
        modules.push(json!({"id": p, "crate": p, "file": src, "line": 1}));
        files.insert(
            src.clone(),
            json!({"sha256": brickie_facts::sha256_hex(text.as_bytes()),
            "compiled": true, "size": text.len(), "lines": 1, "scope": "prod", "module": p,
            "functions": [], "edges": edges, "items": []}),
        );
    }
    let root = ws.dir.path().canonicalize().unwrap();
    let cache = json!({"schema_version": brickie_facts::SCHEMA_VERSION, "tool_version": "0",
        "rust_analyzer": "0", "workspace_root": root.display().to_string(), "git": null,
        "crates": crates, "modules": modules, "files": files});
    ws.write(".brickie/cache.json", &cache.to_string());
    ws
}

fn edge(from: &str, to: &str) -> serde_json::Value {
    json!([{"from": from, "to": to, "kind": "uses", "scope": "prod", "line": 1, "col": 1,
            "end_col": 2, "symbol": "s", "written": "w"}])
}

fn members(file: &str, name: &str, m: &[&str]) -> Op {
    Op::SetBrick {
        file: file.into(),
        name: name.into(),
        desc: None,
        tags: None,
        members: Some(m.iter().map(|s| s.to_string()).collect()),
        path: None,
        independent: None,
    }
}

/// The blocking codes of the draft after `ops`; the publish is refused and writes nothing.
fn blocked(ws: &Ws, ops: Vec<Op>) -> Vec<String> {
    let base = ws.apply(A, ops).unwrap().version.v;
    let d = ws.design();
    let gate = d.publish_check(base).unwrap();
    let e = d.publish(base, A, None).unwrap_err();
    assert!(e.downcast_ref::<Blocked>().is_some(), "{e}");
    assert_eq!(ws.read_published("brickie.toml"), PUB_ROOT);
    assert_eq!(d.forked_from().unwrap(), Some(1), "the draft stays");
    gate.blocking.into_iter().map(|b| b.code).collect()
}

#[test]
fn a_dependency_error_blocks() {
    let ws = measured();
    let codes = blocked(&ws, vec![desc("b", "the b")]);
    assert!(codes.iter().all(|c| c.starts_with("deps/")), "{codes:?}");
    assert!(!codes.is_empty());
}

#[test]
fn a_clean_draft_publishes_one_version() {
    let ws = measured_with(false);
    let base = ws.apply(A, vec![desc("b", "the b")]).unwrap().version.v;
    let d = ws.design();
    let gate = d.publish_check(base).unwrap();
    assert!(gate.ok(), "{:?}", gate.blocking);
    let out = d.publish(base, H, None).unwrap().version;
    assert_eq!((out.v, out.track, out.by), (3, Track::Published, H));
    assert!(
        ws.read_published("brickie.toml")
            .contains("desc = \"the b\"")
    );
    assert_eq!(d.forked_from().unwrap(), None, "the draft is gone");
    // Undoable like any published version: the bytes come back.
    d.undo(3, H, None).unwrap();
    assert_eq!(ws.read_published("brickie.toml"), PUB_ROOT);
}

#[test]
fn publish_overwrites_published_changes_since_the_fork() {
    let ws = measured_with(false);
    let base = ws.apply(A, vec![desc("b", "the b")]).unwrap().version.v;
    // A hand edit moves the published head after the fork.
    ws.write("brickie.toml", &format!("{PUB_ROOT}\n# hand\n"));
    let d = ws.design();
    let out = d.publish(base, A, None).unwrap();
    assert_eq!(out.version.track, Track::Published);
    assert!(
        out.version.note.as_deref().unwrap().contains("overwrote v"),
        "{:?}",
        out.version.note
    );
    let text = ws.read_published("brickie.toml");
    assert!(text.contains("desc = \"the b\""), "{text}");
    assert!(!text.contains("# hand"), "the draft wins: {text}");
    assert_eq!(d.forked_from().unwrap(), None);
}

#[test]
fn a_planned_brick_blocks() {
    let ws = measured_with(false);
    assert_eq!(blocked(&ws, vec![add("c", &["pc"])]), ["design/planned"]);
}

#[test]
fn an_overlap_blocks() {
    let ws = measured_with(false);
    // The overlapping module stays with the first brick, so the new one is also planned.
    assert_eq!(
        blocked(&ws, vec![add("c", &["pb"])]),
        ["config/overlap", "design/planned"]
    );
}

#[test]
fn an_unassigned_module_blocks() {
    let ws = measured();
    let codes = blocked(&ws, vec![members("g/brickie.toml", "x", &["nothing"])]);
    assert!(
        codes.contains(&"config/unassigned".to_string()),
        "{codes:?}"
    );
}

#[test]
fn a_member_outside_the_parent_blocks() {
    let ws = measured();
    let codes = blocked(&ws, vec![members("g/brickie.toml", "x", &["pa", "pb"])]);
    assert!(
        codes.contains(&"config/member-outside-parent".to_string()),
        "{codes:?}"
    );
    let base = ws.head();
    let item = (ws.design().publish_check(base).unwrap().blocking)
        .into_iter()
        .find(|b| b.code == "config/member-outside-parent")
        .unwrap();
    assert_eq!(item.file.as_deref(), Some("g/brickie.toml"));
    assert_eq!(item.member.as_deref(), Some("pb"));
}

#[test]
fn a_stale_cache_blocks() {
    let ws = measured_with(false);
    ws.write("g/pa/src/lib.rs", "// changed\n");
    assert_eq!(blocked(&ws, vec![desc("b", "y")]), ["stale/file"]);
}

#[test]
fn a_denied_link_blocks() {
    let ws = measured();
    let link = Op::AddLink {
        file: "brickie.toml".into(),
        link: crate::RuleSpec {
            from: "b".into(),
            to: "a".into(),
            reason: "r".into(),
            ..Default::default()
        },
    };
    let codes = blocked(&ws, vec![link]);
    assert!(
        codes.contains(&"design/link-denied".to_string()),
        "{codes:?}"
    );
}
