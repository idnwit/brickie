//! Peer brick groups: loading, includes, validation, intra-group errors, external links,
//! `composite/group` exports, `marked_links`, unit-level cycles, complexity, scope filter, and
//! `Todo.groups`.

use crate::check::Filter;
use crate::config::tests::{cache, edge, fixture, func, resolved};
use crate::config::{Brick, Config, load};
use crate::diag::diagnostic;
use std::path::Path;

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, text).unwrap();
}

fn load_str(text: &str) -> anyhow::Result<Config> {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "brickie.toml", text);
    load(&dir.path().join("brickie.toml"))
}

#[test]
fn load_includes_and_unit_helpers() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "extra.toml",
        r#"
[[group]]
name = "from_inc"
tags = ["inc"]
desc = "Included group"

[[brick]]
name = "b_in_inc"
group = "from_root"
members = ["core"]
"#,
    );
    write(
        dir.path(),
        "brickie.toml",
        r#"
include = ["extra.toml"]

[[group]]
name = "from_root"
tags = ["domain"]
desc = "Root group"

[[brick]]
name = "b_in_root"
group = "from_inc"
members = ["web-api"]

[[brick]]
name = "solo"
members = ["web-ui"]

[[rule]]
from = "tag:domain"
to = "tag:inc"
reason = "cross-include tags"
"#,
    );
    let cfg = load(&dir.path().join("brickie.toml")).unwrap();
    assert_eq!(cfg.groups.len(), 2);
    assert!(
        cfg.group("from_root")
            .unwrap()
            .loc
            .contains("brickie.toml:")
    );
    assert!(cfg.group("from_inc").unwrap().loc.contains("extra.toml:"));
    assert_eq!(
        cfg.group_members("from_root")
            .map(|b| b.name.as_str())
            .collect::<Vec<_>>(),
        vec!["b_in_inc"]
    );
    assert!(cfg.unit_by_name("b_in_inc").is_none());
    let u = cfg.unit_by_name("from_root").unwrap();
    assert!(u.is_group);
    assert_eq!(u.tags, &["domain"]);
    assert_eq!(
        cfg.unit_of(cfg.brick("b_in_inc").unwrap()).name,
        "from_root"
    );
    assert_eq!(
        cfg.units().iter().map(|u| u.name).collect::<Vec<_>>(),
        vec!["solo", "from_inc", "from_root"]
    );
    // `unit_of` falls back to the brick itself when `group` names a missing group (never panics).
    let orphan = Brick {
        name: "orphan".into(),
        members: vec![],
        tags: vec![],
        path: None,
        desc: None,
        independent: false,
        group: Some("missing".into()),
        base: Default::default(),
        sub: None,
    };
    let u = cfg.unit_of(&orphan);
    assert!(!u.is_group);
    assert_eq!(u.name, "orphan");
}

#[test]
fn validation_rejects_invalid_group_configs() {
    for bad in ["", "a b", "a/b", "*", "a*b", "root", "tag:x"] {
        let e = format!(
            "{:#}",
            load_str(&format!("[[group]]\nname = \"{bad}\"\n")).unwrap_err()
        );
        assert!(
            e.contains("config/invalid") && e.contains("invalid group name"),
            "{bad}: {e}"
        );
    }
    let dup_group = "[[group]]\nname = \"g\"\n[[group]]\nname = \"g\"\n";
    assert!(format!("{:#}", load_str(dup_group).unwrap_err()).contains("duplicate name"));

    let dup_brick = "[[group]]\nname = \"g\"\n[[brick]]\nname = \"g\"\n";
    assert!(format!("{:#}", load_str(dup_brick).unwrap_err()).contains("duplicate name"));

    let unknown_group = "[[brick]]\nname = \"a\"\ngroup = \"nope\"\n";
    assert!(format!("{:#}", load_str(unknown_group).unwrap_err()).contains("unknown group"));

    let tags_on_member =
        "[[group]]\nname = \"g\"\n[[brick]]\nname = \"a\"\ngroup = \"g\"\ntags = [\"x\"]\n";
    assert!(
        format!("{:#}", load_str(tags_on_member).unwrap_err())
            .contains("tags must be set on the group")
    );

    let indep_on_member =
        "[[group]]\nname = \"g\"\n[[brick]]\nname = \"a\"\ngroup = \"g\"\nindependent = true\n";
    assert!(
        format!("{:#}", load_str(indep_on_member).unwrap_err())
            .contains("grouped bricks cannot be independent")
    );

    // Explicit defaults `tags = []` and `independent = false` are allowed.
    let ok_defaults = "[[group]]\nname = \"g\"\n[[brick]]\nname = \"a\"\ngroup = \"g\"\ntags = []\nindependent = false\n";
    load_str(ok_defaults).unwrap();

    // Targeting grouped member or `group/member` in rule, link, exports, complexity.
    let base = "[[group]]\nname = \"svc\"\n[[brick]]\nname = \"auth\"\ngroup = \"svc\"\n[[brick]]\nname = \"api\"\n";
    for extra in [
        "[[rule]]\nfrom = \"api\"\nto = \"auth\"\nreason = \"r\"\n",
        "[[rule]]\nfrom = \"api\"\nto = \"svc/auth\"\nreason = \"r\"\n",
        "[[link]]\nfrom = \"api\"\nto = \"auth\"\nreason = \"r\"\n",
        "[[link]]\nfrom = \"auth\"\nto = \"api\"\nreason = \"r\"\n",
        "[[link]]\nfrom = \"api\"\nto = \"svc/auth\"\nreason = \"r\"\n",
        "[[complexity]]\nbricks = \"auth\"\ncognitive = 10\n",
    ] {
        let e = format!("{:#}", load_str(&format!("{base}{extra}")).unwrap_err());
        assert!(
            e.contains("target the group name 'svc' instead"),
            "{extra} => {e}"
        );
    }
    for prefix in ["exports = [\"auth\"]\n", "exports = [\"svc/auth\"]\n"] {
        let e = format!("{:#}", load_str(&format!("{prefix}{base}")).unwrap_err());
        assert!(
            e.contains("target the group name 'svc' instead"),
            "{prefix} => {e}"
        );
    }
}

#[test]
fn intra_group_edges_fail_and_external_links_and_marked_links_work() {
    let root = r#"
[[group]]
name = "svc"
tags = ["domain"]

[[brick]]
name = "auth"
group = "svc"
members = ["web-api"]

[[brick]]
name = "billing"
group = "svc"
members = ["web-ui"]

[[brick]]
name = "api"
members = ["core"]

[[brick]]
name = "db"
members = ["web-shared"]

[[rule]]
from = "api"
to = "tag:domain"
reason = "api calls domain services"

[[link]]
from = "api"
to = "svc"
reason = "api uses svc"
"#;
    let (dir, cfg) = fixture(root, "");
    let res = resolved(dir.path(), &cfg).unwrap();
    let ws = dir.path().canonicalize().unwrap();
    let c = cache(
        &ws,
        &[
            ("core", vec![edge("core", "web-api", "prod")], vec![]),
            (
                "web-api",
                vec![
                    edge("web-api", "web-ui", "prod"),
                    edge("web-api", "web-ui", "test"),
                    edge("web-api", "web-shared", "prod"),
                ],
                vec![],
            ),
        ],
    );
    let found: Vec<_> = super::evaluate(&c, &cfg, &res, dir.path())
        .iter()
        .map(|f| diagnostic(f.as_ref()))
        .collect();
    // api -> auth is allowed via api -> svc.
    assert!(
        !found.iter().any(|d| d.message.starts_with("`api`")),
        "{found:#?}"
    );

    // auth -> billing (prod + test) are intra-group violations with no `link add` help.
    let intra: Vec<_> = found
        .iter()
        .filter(|d| d.code == "deps/unlinked" && d.bricks == ["auth", "billing"])
        .collect();
    assert_eq!(intra.len(), 2, "{found:#?}");
    for d in intra {
        assert_eq!(
            d.rule.as_ref().unwrap().name,
            "bricks in group 'svc' cannot depend on each other"
        );
        assert_eq!(d.help.as_deref(), Some("remove the dependency"));
    }

    // auth -> db is unlinked; help suggests `link add svc db`, while bricks() keeps member names.
    let ext = found
        .iter()
        .find(|d| d.code == "deps/unlinked" && d.bricks == ["auth", "db"])
        .unwrap();
    assert!(
        ext.help
            .as_deref()
            .unwrap()
            .contains("brickie design link add svc db"),
        "{:?}",
        ext.help
    );

    // `api -> svc` link is marked used (not planned).
    let marked = super::marked_links(&c, &cfg, &res);
    assert_eq!(marked.len(), 1);
    assert!(!marked[0].planned, "{marked:#?}");
}

#[test]
fn composite_group_exports_links_private_and_help() {
    let root = r#"
[[brick]]
name = "core"
members = ["core"]

[[brick]]
name = "web"
path = "crates/web"

[[rule]]
from = "core"
to = "web"
reason = "core calls web"

[[link]]
from = "core"
to = "web/public_g"
reason = "core calls exported group"
"#;
    let web = r#"
exports = ["public_g"]

[[group]]
name = "public_g"

[[group]]
name = "private_g"

[[brick]]
name = "api"
group = "public_g"
members = ["web-api"]

[[brick]]
name = "ui"
group = "private_g"
members = ["web-ui"]

[[brick]]
name = "shared"
members = ["web-shared"]
"#;
    let (dir, cfg) = fixture(root, web);
    let res = resolved(dir.path(), &cfg).unwrap();
    let ws = dir.path().canonicalize().unwrap();
    let c = cache(
        &ws,
        &[(
            "core",
            vec![
                edge("core", "web-api", "prod"),
                edge("core", "web-ui", "prod"),
            ],
            vec![],
        )],
    );
    let found: Vec<_> = super::evaluate(&c, &cfg, &res, dir.path())
        .iter()
        .map(|f| diagnostic(f.as_ref()))
        .collect();
    // core -> web-api is allowed through `web/public_g`.
    assert!(
        !found
            .iter()
            .any(|d| d.message.contains("`public_g`") || d.message.contains("via `public_g`")),
        "{found:#?}"
    );
    // core -> web-ui triggers deps/private with help naming `private_g`, and deps/unlinked with `web/private_g`.
    let priv_d = found.iter().find(|d| d.code == "deps/private").unwrap();
    assert!(
        priv_d
            .help
            .as_deref()
            .unwrap()
            .contains("brickie design export set private_g"),
        "{:?}",
        priv_d.help
    );
    let unlinked = found.iter().find(|d| d.code == "deps/unlinked").unwrap();
    assert!(
        unlinked
            .help
            .as_deref()
            .unwrap()
            .contains("brickie design link add core web/private_g"),
        "{:?}",
        unlinked.help
    );
    // `core -> web/public_g` link is marked used.
    assert!(super::planned_links(&c, &cfg, &res).is_empty());

    // `to = "web/api"` (grouped member inside composite) is rejected at validation.
    let bad_root = root.replace("web/public_g", "web/api");
    let dir2 = tempfile::tempdir().unwrap();
    write(dir2.path(), "brickie.toml", &bad_root);
    write(dir2.path(), "crates/web/brickie.toml", web);
    let e = format!("{:#}", load(&dir2.path().join("brickie.toml")).unwrap_err());
    assert!(
        e.contains("target the group name 'public_g' instead"),
        "{e}"
    );
}

#[test]
fn grouped_composite_help_omits_entered_child() {
    let root = r#"
[[group]]
name = "apps"

[[brick]]
name = "core"
members = ["core"]

[[brick]]
name = "web"
group = "apps"
path = "crates/web"
"#;
    let web = r#"
exports = ["api"]
[[brick]]
name = "api"
members = ["web-api"]
[[brick]]
name = "ui"
members = ["web-ui"]
[[brick]]
name = "shared"
members = ["web-shared"]
"#;
    let (dir, cfg) = fixture(root, web);
    let res = resolved(dir.path(), &cfg).unwrap();
    let ws = dir.path().canonicalize().unwrap();
    let c = cache(
        &ws,
        &[("core", vec![edge("core", "web-api", "prod")], vec![])],
    );
    let found: Vec<_> = super::evaluate(&c, &cfg, &res, dir.path())
        .iter()
        .map(|f| diagnostic(f.as_ref()))
        .collect();
    let unlinked = found.iter().find(|d| d.code == "deps/unlinked").unwrap();
    let help = unlinked.help.as_deref().unwrap();
    assert!(
        help.contains("brickie design link add core apps --kind"),
        "expected `core apps` without `/api`: {help}"
    );
}

#[test]
fn unit_cycles_owners_complexity_scope_and_todo_groups() {
    let root = r#"
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

[[brick]]
name = "unused"
members = ["web-shared"]

[[complexity]]
bricks = "svc"
cognitive = 5
"#;
    let (dir, cfg) = fixture(root, "");
    let res = resolved(dir.path(), &cfg).unwrap();
    let ws = dir.path().canonicalize().unwrap();
    // auth -> bus and bus -> billing form a `svc <-> bus` cycle; auth -> billing is intra-group.
    let c = cache(
        &ws,
        &[
            (
                "web-api",
                vec![
                    edge("web-api", "core", "prod"),
                    edge("web-api", "web-ui", "prod"),
                ],
                vec![func("auth_fn", "web-api", 12)],
            ),
            ("core", vec![edge("core", "web-ui", "prod")], vec![]),
        ],
    );
    let findings = super::evaluate(&c, &cfg, &res, dir.path());
    let cycle = findings
        .iter()
        .find(|f| f.code() == "deps/cycle")
        .expect("expected unit-level cycle");
    assert_eq!(cycle.bricks(), vec!["bus", "svc"]);
    let owners = cycle.owners();
    for expected in ["root/bus", "root/svc", "root/auth", "root/billing"] {
        assert!(owners.contains(&expected.to_string()), "{owners:#?}");
    }
    // Complexity limit on group `svc` applies to member `auth`.
    assert!(
        findings
            .iter()
            .any(|f| f.code() == "complexity/cognitive" && f.bricks() == ["auth"]),
        "expected complexity finding for auth"
    );

    // Filter::new resolves group path (`svc`) and member path (`auth`), and Todo.groups reports
    // empty_g (not svc) through `check::load`.
    write(
        dir.path(),
        ".brickie/cache.json",
        &serde_json::to_string(&c).unwrap(),
    );
    let loaded = crate::check::load(&dir.path().join("brickie.toml")).unwrap();
    let by_group = Filter::new(&cfg, &["svc".into()]).unwrap().apply(&loaded);
    assert!(by_group.shown.iter().any(|d| d.code == "deps/cycle"));
    assert!(
        by_group
            .shown
            .iter()
            .any(|d| d.code == "complexity/cognitive")
    );

    let by_member = Filter::new(&cfg, &["auth".into()]).unwrap().apply(&loaded);
    assert!(by_member.shown.iter().any(|d| d.code == "deps/cycle"));

    assert_eq!(loaded.todo.groups, vec![vec!["empty_g".to_string()]]);
    assert!(
        loaded
            .todo
            .lines()
            .contains(&"planned group root/empty_g has no bricks yet".to_string())
    );
}
