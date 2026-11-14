//! Versions: stale bases, concurrent writers, hand edits, invalid designs.

use super::*;
use crate::{By, Stale};

#[test]
fn first_read_records_v1() {
    let ws = Ws::new();
    let h = ws.design().head().unwrap();
    assert_eq!((h.v, h.by, h.full), (1, By::External, true));
    assert_eq!(ws.head(), 1, "a second read adds nothing");
}

#[test]
fn stale_base_writes_nothing_and_shows_the_changes() {
    let ws = Ws::new();
    let d = ws.design();
    d.apply(1, H, Some("store".into()), vec![add("c", &["pc"])])
        .unwrap();
    let before = ws.read("brickie.toml");
    let e = d.apply(1, A, None, vec![desc("a", "x")]).unwrap_err();
    let stale = e.downcast_ref::<Stale>().expect("a stale error");
    assert_eq!(stale.head, 2);
    let m = &stale.message;
    assert!(
        m.starts_with("error[design/stale]: design is at v2 (by human"),
        "{m}"
    );
    assert!(
        m.contains("your edit was based on v1. Nothing was written."),
        "{m}"
    );
    assert!(m.contains("v2 by human (store):"), "{m}");
    assert!(m.contains("+ brickie.toml: brick c"), "{m}");
    assert!(m.ends_with("retry with --base 2."), "{m}");
    assert_eq!(ws.read("brickie.toml"), before);
    let unknown = d.apply(9, A, None, vec![desc("a", "x")]).unwrap_err();
    assert!(
        unknown.to_string().contains("base v9 is unknown"),
        "{unknown}"
    );
}

#[test]
fn two_concurrent_writers_exactly_one_wins() {
    let ws = Ws::new();
    let base = ws.head();
    let config = ws.config();
    let threads: Vec<_> = ["x", "y"]
        .into_iter()
        .map(|n| {
            let config = config.clone();
            std::thread::spawn(move || {
                Design::at(&config)
                    .unwrap()
                    .apply(base, A, None, vec![add(n, &[n])])
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    let ok = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(ok, 1, "{results:?}");
    let err = results.iter().find_map(|r| r.as_ref().err()).unwrap();
    assert!(err.downcast_ref::<Stale>().is_some(), "{err}");
    assert_eq!(ws.head(), base + 1);
}

#[test]
fn hand_edit_becomes_an_external_version() {
    let ws = Ws::new();
    let base = ws.head();
    let text = ws
        .read("brickie.toml")
        .replace("name = \"b\"", "name = \"bee\"");
    ws.write("brickie.toml", &text);
    let e = ws
        .design()
        .apply(base, A, None, vec![desc("a", "x")])
        .unwrap_err();
    assert!(e.downcast_ref::<Stale>().is_some(), "{e}");
    let log = ws.design().log().unwrap();
    let last = log.last().unwrap();
    assert_eq!((last.v, last.by), (2, By::External));
    assert!(
        last.diff.contains(&"+ brickie.toml: brick bee".to_string()),
        "{:?}",
        last.diff
    );
    assert!(
        last.diff.contains(&"- brickie.toml: brick b".to_string()),
        "{:?}",
        last.diff
    );
}

#[test]
fn invalid_design_is_rejected_and_files_untouched() {
    let ws = Ws::new();
    let before = ws.read("brickie.toml");
    let bad = Op::SetDeps {
        file: "brickie.toml".into(),
        default: None,
        cycles: None,
    };
    let rule = Op::AddRule {
        file: "brickie.toml".into(),
        rule: crate::RuleSpec {
            from: "nope".into(),
            to: "b".into(),
            action: "allow".into(),
            ..Default::default()
        },
    };
    let e = format!("{:#}", ws.apply(A, vec![bad, rule]).unwrap_err());
    assert!(
        e.contains("design/invalid") && e.contains("nothing was written"),
        "{e}"
    );
    assert_eq!(ws.read("brickie.toml"), before);
    assert_eq!(ws.head(), 1);
}

#[test]
fn a_write_keeps_comments_and_records_ops_and_diff() {
    let ws = Ws::new();
    let out = ws.apply(A, vec![desc("a", "the core")]).unwrap();
    assert!(out.changed);
    let text = ws.read("brickie.toml");
    assert!(
        text.starts_with("# the design") && text.contains("# first"),
        "{text}"
    );
    let v = out.version;
    assert_eq!((v.v, v.by, v.ops.len()), (2, A, 1));
    assert_eq!(
        v.diff,
        vec!["~ brickie.toml: brick a: desc: (none) -> \"the core\""]
    );
    let same = ws.apply(A, vec![desc("a", "the core")]).unwrap();
    assert!(!same.changed && same.version.v == 2);
}

#[test]
fn history_keeps_at_most_cap_versions() {
    let ws = Ws::new();
    for i in 0..crate::CAP + 3 {
        ws.apply(A, vec![desc("a", &format!("d{i}"))]).unwrap();
    }
    let d = ws.design();
    let log = d.log().unwrap();
    assert_eq!(log.len(), crate::CAP);
    assert!(log[0].full);
    let first = d.state(log[0].v).unwrap();
    assert!(String::from_utf8_lossy(&first["brickie.toml"]).contains("desc = \"d3\""));
}

#[test]
fn dry_run_reports_and_writes_nothing() {
    let ws = Ws::new();
    let d = ws.design();
    let base = d.head().unwrap().v;
    let before = ws.read("brickie.toml");
    let plan = d.dry_run(base, vec![add("c", &["pc"])]).unwrap();
    assert!(
        plan.diff.iter().any(|l| l.contains("brick c")),
        "{:?}",
        plan.diff
    );
    assert_eq!(ws.read("brickie.toml"), before);
    assert_eq!(d.log().unwrap().len(), 1);
    assert!(d.dry_run(base + 1, vec![add("c", &[])]).is_err());
}
