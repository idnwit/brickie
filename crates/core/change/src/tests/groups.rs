//! Peer brick groups (`[[group]]`): add, set, rename (across includes and parent
//! `composite/group` links), remove, brick grouping, move/unnest guards, diff labels, publish
//! gate, and `link_options`/`link_check`.

use super::*;
use crate::{Blocked, BrickKind, GroupSpec, Link, RuleSpec};
use serde_json::json;

fn add_group(file: &str, name: &str, tags: &[&str], desc: Option<&str>) -> Op {
    Op::AddGroup {
        file: file.into(),
        group: GroupSpec {
            name: name.into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            desc: desc.map(String::from),
        },
    }
}

fn set_group_on_brick(file: &str, name: &str, group: &str) -> Op {
    Op::SetBrick {
        file: file.into(),
        name: name.into(),
        desc: None,
        tags: None,
        members: None,
        path: None,
        independent: None,
        group: Some(group.into()),
    }
}

#[test]
fn add_set_and_remove_group_with_diff_labels() {
    let ws = Ws::new();
    let out = ws
        .apply(
            A,
            vec![add_group(
                "brickie.toml",
                "services",
                &["domain"],
                Some("Domain services"),
            )],
        )
        .unwrap();
    assert!(
        out.version
            .diff
            .iter()
            .any(|l| l == "+ brickie.toml: group services"),
        "{:?}",
        out.version.diff
    );
    let show = ws.design().show_design().unwrap();
    assert_eq!(show.root.groups.len(), 1);
    assert_eq!(show.root.groups[0].name, "services");
    assert_eq!(show.root.groups[0].tags, vec!["domain"]);
    assert_eq!(show.root.groups[0].desc.as_deref(), Some("Domain services"));
    assert!(show.root.groups[0].members.is_empty());

    // Update tags and clear desc.
    let out = ws
        .apply(
            A,
            vec![Op::SetGroup {
                file: "brickie.toml".into(),
                name: "services".into(),
                tags: Some(vec![]),
                desc: Some(String::new()),
            }],
        )
        .unwrap();
    assert!(
        out.version
            .diff
            .iter()
            .any(|l| l.starts_with("~ brickie.toml: group services:")),
        "{:?}",
        out.version.diff
    );
    let text = ws.read("brickie.toml");
    assert!(!text.contains("Domain services"), "{text}");

    // Remove unused group succeeds.
    let out = ws
        .apply(
            A,
            vec![Op::RemoveGroup {
                file: "brickie.toml".into(),
                name: "services".into(),
            }],
        )
        .unwrap();
    assert!(
        out.version
            .diff
            .iter()
            .any(|l| l == "- brickie.toml: group services"),
        "{:?}",
        out.version.diff
    );
    assert!(!ws.read("brickie.toml").contains("[[group]]"));
}

#[test]
fn set_brick_group_enforces_rules_and_ungroups_with_empty_string() {
    let ws = Ws::new();
    ws.apply(
        A,
        vec![
            add_group("brickie.toml", "services", &["domain"], None),
            add("c", &["pc"]),
        ],
    )
    .unwrap();

    // `a` has `tags = ["core"]` and direct rule `a -> b`: grouping it must fail.
    let err = format!(
        "{:#}",
        ws.apply(A, vec![set_group_on_brick("brickie.toml", "a", "services")])
            .unwrap_err()
    );
    assert!(err.contains("design/invalid"), "{err}");

    // `c` has no tags, not independent, no direct refs: grouping it succeeds.
    ws.apply(A, vec![set_group_on_brick("brickie.toml", "c", "services")])
        .unwrap();
    assert!(ws.read("brickie.toml").contains("group = \"services\""));
    let show = ws.design().show_design().unwrap();
    assert_eq!(show.root.groups[0].members, vec!["c"]);
    let c_view = show.root.bricks.iter().find(|b| b.name == "c").unwrap();
    assert_eq!(c_view.group.as_deref(), Some("services"));

    // Ungrouping with `Some("")` removes the `group` key.
    ws.apply(A, vec![set_group_on_brick("brickie.toml", "c", "")])
        .unwrap();
    assert!(!ws.read("brickie.toml").contains("group ="));
}

#[test]
fn rename_group_rewrites_across_includes_and_parent_composite_links() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        r#"[[brick]]
name = "api"
members = ["p-api"]

[[brick]]
name = "core"
path = "core"

[[rule]]
from = "api"
to = "core"
reason = "api calls core"

[[link]]
from = "api"
to = "core/services"
reason = "api calls core services"
"#,
    );
    ws.write(
        "core/brickie.toml",
        r#"include = ["extra.toml"]
exports = ["services"]

[[group]]
name = "services"
tags = ["domain"]

[[brick]]
name = "auth"
group = "services"
members = ["p-auth"]

[[brick]]
name = "db"
members = ["p-db"]

[[rule]]
from = "services"
to = "db"
reason = "services use db"

[[link]]
from = "services"
to = "db"
reason = "services query db"

[[complexity]]
bricks = "services"
cognitive = 20
"#,
    );
    ws.write(
        "core/extra.toml",
        r#"[[brick]]
name = "billing"
group = "services"
members = ["p-billing"]
"#,
    );

    // Resolve rejects `--in core/services` (a group, not a composite).
    let d = ws.design();
    assert!(
        d.level_file(&["core".into(), "services".into()]).is_err(),
        "group cannot own a level"
    );
    assert_eq!(
        d.group_file(&["core".into(), "services".into()]).unwrap(),
        ("core/brickie.toml".into(), "services".into())
    );
    assert_eq!(
        d.unit_file(&["core".into(), "services".into()]).unwrap(),
        ("core/brickie.toml".into(), "services".into())
    );
    assert!(
        d.unit_file(&["core".into(), "auth".into()]).is_err(),
        "grouped member is not a standalone unit"
    );

    // Rename `services` -> `domain` inside `core`.
    ws.apply(
        A,
        vec![Op::RenameGroup {
            file: "core/brickie.toml".into(),
            name: "services".into(),
            new_name: "domain".into(),
        }],
    )
    .unwrap();

    let core = ws.read("core/brickie.toml");
    assert!(core.contains("name = \"domain\""), "{core}");
    assert!(core.contains("group = \"domain\""), "{core}");
    assert!(core.contains("exports = [\"domain\"]"), "{core}");
    assert!(core.contains("from = \"domain\""), "{core}");
    assert!(core.contains("bricks = \"domain\""), "{core}");
    assert!(!core.contains("= \"services\""), "{core}");

    let extra = ws.read("core/extra.toml");
    assert!(extra.contains("group = \"domain\""), "{extra}");
    assert!(!extra.contains("services"), "{extra}");

    let root = ws.read("brickie.toml");
    assert!(root.contains("to = \"core/domain\""), "{root}");
    assert!(!root.contains("core/services"), "{root}");

    // Config validates cleanly after rename.
    ws.design().config().unwrap();
}

#[test]
fn remove_group_refuses_when_referenced_locally_or_by_parent_link() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        r#"[[brick]]
name = "api"
members = ["p-api"]

[[brick]]
name = "core"
path = "core"

[[rule]]
from = "api"
to = "core"
reason = "api calls core"

[[link]]
from = "api"
to = "core/services"
reason = "api calls core services"
"#,
    );
    ws.write(
        "core/brickie.toml",
        r#"exports = ["services"]

[[group]]
name = "services"

[[brick]]
name = "auth"
group = "services"
members = ["p-auth"]
"#,
    );

    let rm = || Op::RemoveGroup {
        file: "core/brickie.toml".into(),
        name: "services".into(),
    };

    // Refused: brick `auth` still references `services`.
    let e = format!("{:#}", ws.apply(A, vec![rm()]).unwrap_err());
    assert!(e.contains("design/invalid") && e.contains("auth"), "{e}");

    // Ungroup `auth`: still refused because `exports` and parent link reference `services`.
    ws.apply(A, vec![set_group_on_brick("core/brickie.toml", "auth", "")])
        .unwrap();
    let e = format!("{:#}", ws.apply(A, vec![rm()]).unwrap_err());
    assert!(e.contains("design/invalid"), "{e}");

    // Removing export is blocked by parent link `core/services`.
    assert!(
        ws.apply(
            A,
            vec![Op::RemoveExport {
                file: "core/brickie.toml".into(),
                brick: "services".into(),
            }]
        )
        .is_err()
    );

    // Remove parent link + export + group in one write succeeds.
    ws.apply(
        A,
        vec![
            Op::RemoveLink {
                file: "brickie.toml".into(),
                key: crate::RuleKey {
                    from: "api".into(),
                    to: "core/services".into(),
                    ..Default::default()
                },
            },
            Op::RemoveExport {
                file: "core/brickie.toml".into(),
                brick: "services".into(),
            },
            rm(),
        ],
    )
    .unwrap();
    assert!(!ws.read("core/brickie.toml").contains("[[group]]"));
}

#[test]
fn move_and_unnest_respect_groups() {
    let ws = Ws::new();
    ws.apply(
        A,
        vec![
            add_group("brickie.toml", "services", &[], None),
            Op::AddBrick {
                file: "brickie.toml".into(),
                brick: BrickSpec {
                    name: "svc_a".into(),
                    kind: Some(BrickKind::Composite),
                    group: Some("services".into()),
                    ..Default::default()
                },
            },
            Op::AddBrick {
                file: "svc_a/brickie.toml".into(),
                brick: BrickSpec {
                    name: "inner".into(),
                    members: vec!["p-inner".into()],
                    ..Default::default()
                },
            },
            add("free_b", &["p-free"]),
        ],
    )
    .unwrap();

    // Moving a grouped brick is refused.
    let e = format!(
        "{:#}",
        ws.apply(
            A,
            vec![Op::MoveBrick {
                file: "brickie.toml".into(),
                name: "svc_a".into(),
                into_file: "brickie.toml".into(),
                into: Some("b".into()),
                path: None,
            }]
        )
        .unwrap_err()
    );
    assert!(e.contains("design/invalid") && e.contains("ungroup"), "{e}");

    // Moving `--into <group>` is refused.
    let e = format!(
        "{:#}",
        ws.apply(
            A,
            vec![Op::MoveBrick {
                file: "brickie.toml".into(),
                name: "free_b".into(),
                into_file: "brickie.toml".into(),
                into: Some("services".into()),
                path: None,
            }]
        )
        .unwrap_err()
    );
    assert!(e.contains("design/invalid") && e.contains("group"), "{e}");

    // Moving `a` (which has direct rule `a -> b`) into grouped composite `svc_a` is refused.
    let e = format!(
        "{:#}",
        ws.apply(
            A,
            vec![Op::MoveBrick {
                file: "brickie.toml".into(),
                name: "a".into(),
                into_file: "brickie.toml".into(),
                into: Some("svc_a".into()),
                path: None,
            }]
        )
        .unwrap_err()
    );
    assert!(e.contains("design/invalid"), "{e}");

    // Unnesting a grouped composite lifts its children as ungrouped bricks.
    ws.apply(
        A,
        vec![Op::Unnest {
            file: "brickie.toml".into(),
            name: "svc_a".into(),
        }],
    )
    .unwrap();
    let cfg = ws.design().config().unwrap();
    assert!(cfg.brick("inner").unwrap().group.is_none());
    assert_eq!(
        cfg.brick("svc_a").unwrap().group.as_deref(),
        Some("services")
    );
}

#[test]
fn unnest_lifts_child_groups_and_rejects_name_collisions() {
    let ws = Ws::new();
    ws.apply(
        A,
        vec![
            add_group("brickie.toml", "shared", &[], None),
            Op::AddBrick {
                file: "brickie.toml".into(),
                brick: BrickSpec {
                    name: "comp".into(),
                    kind: Some(BrickKind::Composite),
                    ..Default::default()
                },
            },
            add_group("comp/brickie.toml", "subgrp", &["t"], None),
            Op::AddBrick {
                file: "comp/brickie.toml".into(),
                brick: BrickSpec {
                    name: "x".into(),
                    members: vec!["px".into()],
                    group: Some("subgrp".into()),
                    ..Default::default()
                },
            },
        ],
    )
    .unwrap();

    // Unnest lifts `subgrp` cleanly.
    ws.apply(
        A,
        vec![Op::Unnest {
            file: "brickie.toml".into(),
            name: "comp".into(),
        }],
    )
    .unwrap();
    let cfg = ws.design().config().unwrap();
    assert!(cfg.group("subgrp").is_some());
    assert_eq!(cfg.brick("x").unwrap().group.as_deref(), Some("subgrp"));

    // Now test collision: composite contains a group named `shared` which already exists in root.
    ws.apply(
        A,
        vec![
            Op::MakeComposite {
                file: "brickie.toml".into(),
                name: "comp".into(),
                path: "comp2".into(),
            },
            add_group("comp2/brickie.toml", "shared", &[], None),
        ],
    )
    .unwrap();
    let e = format!(
        "{:#}",
        ws.apply(
            A,
            vec![Op::Unnest {
                file: "brickie.toml".into(),
                name: "comp".into(),
            }]
        )
        .unwrap_err()
    );
    assert!(
        e.contains("design/invalid") && e.contains("collides"),
        "{e}"
    );
}

#[test]
fn link_options_and_link_check_operate_on_units_including_groups() {
    let ws = Ws::new();
    ws.apply(
        A,
        vec![
            add_group("brickie.toml", "services", &["domain"], None),
            Op::AddBrick {
                file: "brickie.toml".into(),
                brick: BrickSpec {
                    name: "auth".into(),
                    members: vec!["p-auth".into()],
                    group: Some("services".into()),
                    ..Default::default()
                },
            },
            Op::AddRule {
                file: "brickie.toml".into(),
                rule: RuleSpec {
                    from: "a".into(),
                    to: "tag:domain".into(),
                    reason: "a calls domain".into(),
                    ..Default::default()
                },
            },
        ],
    )
    .unwrap();

    let d = ws.design();
    // `link_options` for `a` includes `services` (allowed) and excludes grouped member `auth`.
    let opts = d.link_options(&["a".into()]).unwrap();
    assert!(
        opts.iter().any(|o| o.to == "services" && o.allowed),
        "{opts:?}"
    );
    assert!(
        opts.iter().all(|o| o.to != "auth"),
        "grouped member must not appear: {opts:?}"
    );

    // `link_options` for `services` works, while `link_options` for grouped member `auth` fails.
    assert!(d.link_options(&["services".into()]).is_ok());
    assert!(d.link_options(&["auth".into()]).is_err());

    // `link_check` allows `a -> services` and rejects `a -> auth`.
    let chk = d
        .link_check(
            &[],
            &Link {
                from: "a",
                to: "services",
                kinds: &[],
                scope: "all",
            },
        )
        .unwrap();
    assert!(chk.allowed, "{chk:?}");
    assert!(
        d.link_check(
            &[],
            &Link {
                from: "a",
                to: "auth",
                kinds: &[],
                scope: "all",
            },
        )
        .is_err()
    );

    // `layout_score` scores links to groups without panicking or dropping them.
    ws.apply(
        A,
        vec![Op::AddLink {
            file: "brickie.toml".into(),
            link: RuleSpec {
                from: "a".into(),
                to: "services".into(),
                reason: "a uses services".into(),
                ..Default::default()
            },
        }],
    )
    .unwrap();
    let view = ws.design().show_design().unwrap();
    assert_eq!(view.root.layout.unwrap().edges, 1);
}

#[test]
fn publish_gate_blocks_on_empty_group() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        "[[brick]]\nname = \"a\"\nmembers = [\"pa\"]\n",
    );
    ws.write("Cargo.toml", "[workspace]\nmembers = [\"crates/pa\"]\n");
    ws.write("crates/pa/Cargo.toml", "[package]\nname = \"pa\"\n");
    let src = "crates/pa/src/lib.rs";
    let text = "// pa\n";
    ws.write(src, text);
    let root = ws.dir.path().canonicalize().unwrap();
    let cache = json!({
        "schema_version": brickie_facts::SCHEMA_VERSION,
        "tool_version": "0",
        "rust_analyzer": "0",
        "workspace_root": root.display().to_string(),
        "git": null,
        "crates": [{"id": "pa", "package": "pa", "version": "0", "target": "pa", "kind": "lib", "root": src, "dir": "crates/pa"}],
        "modules": [{"id": "pa", "crate": "pa", "file": src, "line": 1}],
        "files": {
            src: {"sha256": brickie_facts::sha256_hex(text.as_bytes()), "compiled": true, "size": text.len(), "lines": 1, "scope": "prod", "module": "pa", "functions": [], "edges": [], "items": []}
        }
    });
    ws.write(".brickie/cache.json", &cache.to_string());

    let base = ws
        .apply(A, vec![add_group("brickie.toml", "empty", &[], None)])
        .unwrap()
        .version
        .v;
    let d = ws.design();
    let gate = d.publish_check(base).unwrap();
    assert!(!gate.ok());
    assert!(
        gate.blocking.iter().any(|b| b.code == "design/planned"
            && b.message == "planned group root/empty has no bricks yet"),
        "{:?}",
        gate.blocking
    );
    let err = d.publish(base, A, None).unwrap_err();
    assert!(err.downcast_ref::<Blocked>().is_some(), "{err}");
}

#[test]
fn group_ops_via_level_root_edit_included_file() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        r#"include = ["groups.toml"]

[[brick]]
name = "a"
members = ["pa"]
group = "services"
"#,
    );
    ws.write(
        "groups.toml",
        r#"[[group]]
name = "services"
tags = ["domain"]
"#,
    );

    // `SetGroup` targeting the level root edits the `[[group]]` declared in `groups.toml`.
    ws.apply(
        A,
        vec![Op::SetGroup {
            file: "brickie.toml".into(),
            name: "services".into(),
            tags: Some(vec!["core".into()]),
            desc: Some("Core services".into()),
        }],
    )
    .unwrap();
    let groups_toml = ws.read("groups.toml");
    assert!(groups_toml.contains("\"core\""), "{groups_toml}");
    assert!(groups_toml.contains("Core services"), "{groups_toml}");

    // `RenameGroup` targeting the level root renames `services` in `groups.toml` and `brick.group`.
    ws.apply(
        A,
        vec![Op::RenameGroup {
            file: "brickie.toml".into(),
            name: "services".into(),
            new_name: "domain".into(),
        }],
    )
    .unwrap();
    assert!(ws.read("groups.toml").contains("name = \"domain\""));
    assert!(ws.read("brickie.toml").contains("group = \"domain\""));

    // Ungroup `a`, then `RemoveGroup` via the level root removes `[[group]]` from `groups.toml`.
    ws.apply(
        A,
        vec![
            set_group_on_brick("brickie.toml", "a", ""),
            Op::RemoveGroup {
                file: "brickie.toml".into(),
                name: "domain".into(),
            },
        ],
    )
    .unwrap();
    assert!(!ws.read("groups.toml").contains("[[group]]"));
}

#[test]
fn parent_composite_group_link_in_included_file_is_updated_and_guarded() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        r#"include = ["links.toml"]

[[brick]]
name = "api"
members = ["p-api"]

[[brick]]
name = "core"
path = "core"

[[rule]]
from = "api"
to = "core"
reason = "api calls core"
"#,
    );
    ws.write(
        "links.toml",
        r#"[[link]]
from = "api"
to = "core/services"
reason = "api calls core services"
"#,
    );
    ws.write(
        "core/brickie.toml",
        r#"exports = ["services"]

[[group]]
name = "services"

[[brick]]
name = "auth"
group = "services"
members = ["p-auth"]
"#,
    );

    // Renaming `services` -> `domain` in `core/brickie.toml` rewrites `links.toml`.
    ws.apply(
        A,
        vec![Op::RenameGroup {
            file: "core/brickie.toml".into(),
            name: "services".into(),
            new_name: "domain".into(),
        }],
    )
    .unwrap();
    let links = ws.read("links.toml");
    assert!(links.contains("to = \"core/domain\""), "{links}");
    assert!(!links.contains("core/services"), "{links}");

    // Unexporting `domain` is blocked by the link in `links.toml`.
    let e = format!(
        "{:#}",
        ws.apply(
            A,
            vec![Op::RemoveExport {
                file: "core/brickie.toml".into(),
                brick: "domain".into(),
            }]
        )
        .unwrap_err()
    );
    assert!(
        e.contains("design/invalid") && e.contains("links.toml"),
        "{e}"
    );

    // Unnesting `core` is blocked by the `core/domain` link in `links.toml`.
    let e = format!(
        "{:#}",
        ws.apply(
            A,
            vec![Op::Unnest {
                file: "brickie.toml".into(),
                name: "core".into(),
            }]
        )
        .unwrap_err()
    );
    assert!(
        e.contains("design/invalid") && e.contains("links.toml"),
        "{e}"
    );
}
