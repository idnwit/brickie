use super::*;
use crate::{Op, PlanAction, RuleSpec, Verdict};

#[test]
fn publish_writes_configs_scaffolds_and_closes() {
    let ws = Ws::new();
    ws.write("crates/pa/Cargo.toml", "keep me");
    let c = ws.open();
    c.record(H, add("n", &["path:crates/n/"]), None).unwrap();
    c.record(H, add("old", &["path:crates/pa/"]), None).unwrap();
    c.record(H, desc("a", "core"), None).unwrap();
    let plan = c.publish_plan().unwrap();
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    let paths: Vec<_> = plan
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.action))
        .collect();
    assert!(
        paths.contains(&("brickie.toml", PlanAction::Write)),
        "{paths:?}"
    );
    assert!(
        paths.contains(&("crates/n/Cargo.toml", PlanAction::Create)),
        "{paths:?}"
    );
    assert!(
        paths.contains(&("crates/n/src/lib.rs", PlanAction::Create)),
        "{paths:?}"
    );
    assert!(
        paths.contains(&("Cargo.toml", PlanAction::Edit)),
        "{paths:?}"
    );
    assert!(
        !paths.iter().any(|(p, _)| p.starts_with("crates/pa")),
        "{paths:?}"
    );
    c.publish().unwrap();
    let root = ws.read("brickie.toml");
    assert!(
        root.contains("name = \"n\"") && root.contains("# the design"),
        "{root}"
    );
    assert!(ws.read("crates/n/Cargo.toml").contains("name = \"n\""));
    assert!(ws.read("Cargo.toml").contains("crates/n"));
    assert_eq!(ws.read("crates/pa/Cargo.toml"), "keep me");
    assert!(!Change::is_open(&ws.config()));
    assert!(!exists(&ws.path(".brickie/change")));
}

#[test]
fn publish_refuses_proposed_outside_edits_and_invalid_configs() {
    let ws = Ws::new();
    let c = ws.open();
    let p = c.record(A, desc("a", "x"), None).unwrap();
    let blockers = c.publish_plan().unwrap().blockers;
    assert!(
        blockers.iter().any(|b| b.starts_with("change/proposed")),
        "{blockers:?}"
    );
    c.review(H, p.seq, Verdict::Accept, None).unwrap();
    assert!(c.publish_plan().unwrap().blockers.is_empty());
    // A rule naming an unknown brick: the result does not load.
    let rule = RuleSpec {
        from: "a".into(),
        to: "ghost".into(),
        action: "deny".into(),
        ..RuleSpec::default()
    };
    let r = c
        .record(
            H,
            Op::AddRule {
                file: "brickie.toml".into(),
                rule,
            },
            None,
        )
        .unwrap();
    let blockers = c.publish_plan().unwrap().blockers;
    assert!(
        blockers.iter().any(|b| b.starts_with("change/invalid")),
        "{blockers:?}"
    );
    c.review(H, r.seq, Verdict::Reject, None).unwrap();
    // Someone edits the toml by hand.
    ws.write("brickie.toml", &format!("{ROOT}\n# edited\n"));
    let blockers = c.publish_plan().unwrap().blockers;
    assert!(
        blockers
            .iter()
            .any(|b| b.starts_with("change/outside-edit")),
        "{blockers:?}"
    );
    let e = format!("{:#}", ws.open_existing().publish().unwrap_err());
    assert!(e.contains("publish refused"), "{e}");
    assert!(ws.read("brickie.toml").contains("# edited"));
    ws.open_existing().discard().unwrap();
    assert!(!Change::is_open(&ws.config()));
}

#[test]
fn scaffolds_a_module_member_into_a_measured_crate() {
    let ws = Ws::new();
    ws.write("crates/pa/src/lib.rs", "pub fn x() {}");
    ws.write(
        ".brickie/cache.json",
        r#"{"schema_version":6,"tool_version":"0","rust_analyzer":"0","workspace_root":"","git":null,
            "crates":[{"id":"pa","package":"pa","version":"0","target":"pa","kind":"lib",
            "root":"crates/pa/src/lib.rs","dir":"crates/pa"}],"modules":[],"files":{}}"#,
    );
    let c = ws.open();
    c.record(H, add("eng", &["pa::engine"]), None).unwrap();
    c.record(H, add("deep", &["pa::nope::x"]), None).unwrap();
    let plan = c.publish_plan().unwrap();
    let skip = plan
        .files
        .iter()
        .find(|f| f.action == PlanAction::Skip)
        .unwrap();
    assert_eq!(skip.path, "pa::nope::x");
    c.publish().unwrap();
    assert_eq!(
        ws.read("crates/pa/src/lib.rs"),
        "pub fn x() {}\npub mod engine;\n"
    );
    assert!(exists(&ws.path("crates/pa/src/engine/mod.rs")));
}
