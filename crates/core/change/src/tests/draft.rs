//! The draft: created by the first write, tagged by track, discarded, rebased.

use super::*;
use crate::{Stale, Track};

#[test]
fn the_first_write_forks_a_draft_and_leaves_the_published_files() {
    let ws = Ws::new();
    let out = ws.apply(A, vec![desc("a", "x")]).unwrap();
    let v = &out.version;
    assert_eq!(
        (v.v, v.track, v.forked_from, v.full),
        (2, Track::Draft, Some(1), true)
    );
    assert_eq!(
        ws.read_published("brickie.toml"),
        ROOT,
        "published untouched"
    );
    assert!(ws.read("brickie.toml").contains("desc = \"x\""));
    let d = ws.design();
    assert_eq!(d.forked_from().unwrap(), Some(1));
    assert_eq!((d.head().unwrap().v, d.published_head().unwrap().v), (2, 1));
    let show = d.show_design().unwrap();
    assert_eq!(
        (show.track, show.forked_from, show.v),
        (Track::Draft, Some(1), 2)
    );
    // The next write is based on the draft head.
    let next = ws.apply(A, vec![desc("b", "y")]).unwrap().version;
    assert_eq!(
        (next.v, next.track, next.forked_from, next.full),
        (3, Track::Draft, None, false)
    );
}

#[test]
fn a_write_that_changes_nothing_creates_no_draft() {
    let ws = Ws::new();
    let out = ws.apply(A, vec![desc("a", "x")]).unwrap();
    ws.design().discard(out.version.v, A).unwrap();
    let ws = Ws::new();
    let same = Op::SetBrick {
        file: "brickie.toml".into(),
        name: "b".into(),
        desc: None,
        tags: None,
        members: Some(vec!["pb".into()]),
        path: None,
        independent: None,
    };
    assert!(!ws.apply(A, vec![same]).unwrap().changed);
    assert_eq!(ws.design().forked_from().unwrap(), None);
}

#[test]
fn versions_share_one_counter_and_hand_edits_stay_published() {
    let ws = Ws::new();
    ws.apply(A, vec![desc("a", "x")]).unwrap();
    ws.write(
        "brickie.toml",
        &ROOT.replace("name = \"b\"", "name = \"b\"\ndesc = \"hand\""),
    );
    let d = ws.design();
    let p = d.published_head().unwrap();
    assert_eq!((p.v, p.track, p.by), (3, Track::Published, By::External));
    assert_eq!(d.head().unwrap().v, 2, "the draft head is unchanged");
    let v = ws.apply(A, vec![desc("a", "z")]).unwrap().version;
    assert_eq!((v.v, v.track), (4, Track::Draft));
    let tracks: Vec<_> = d.log().unwrap().iter().map(|e| (e.v, e.track)).collect();
    use Track::{Draft, Published};
    assert_eq!(
        tracks,
        [(1, Published), (2, Draft), (3, Published), (4, Draft)]
    );
    // Each track's state replays only its own versions.
    assert!(!String::from_utf8_lossy(&d.state(4).unwrap()["brickie.toml"]).contains("hand"));
}

#[test]
fn discard_drops_the_draft_and_the_next_write_forks_again() {
    let ws = Ws::new();
    let v = ws.apply(A, vec![desc("a", "x")]).unwrap().version.v;
    let d = ws.design();
    assert!(
        d.discard(v + 1, A)
            .unwrap_err()
            .downcast_ref::<Stale>()
            .is_some()
    );
    let out = d.discard(v, A).unwrap().version;
    assert_eq!((out.track, out.full), (Track::Draft, true));
    assert!(out.note.unwrap().starts_with("discard draft"));
    assert_eq!(d.forked_from().unwrap(), None);
    assert!(!ws.path(".brickie/draft").exists());
    assert_eq!(ws.read("brickie.toml"), ROOT);
    assert!(
        d.undo(out.v, A, Some(v)).is_err(),
        "a discarded draft cannot be undone"
    );
    let again = ws.apply(A, vec![desc("b", "y")]).unwrap().version;
    assert_eq!((again.forked_from, again.v), (Some(1), out.v + 1));
    assert!(!ws.read("brickie.toml").contains("desc = \"x\""));
}

#[test]
fn undo_of_the_fork_returns_the_draft_to_the_published_files() {
    let ws = Ws::new();
    ws.apply(A, vec![desc("a", "x")]).unwrap();
    let out = ws.design().undo(2, A, None).unwrap().version;
    assert_eq!((out.v, out.track, out.reverts), (3, Track::Draft, Some(2)));
    assert_eq!(ws.read("brickie.toml"), ROOT);
    assert_eq!(ws.read_published("brickie.toml"), ROOT);
}

#[test]
fn rebase_replays_the_draft_onto_the_new_published_head() {
    let ws = Ws::new();
    ws.apply(A, vec![add("c", &["pc"])]).unwrap();
    ws.apply(A, vec![desc("a", "x")]).unwrap();
    ws.design().undo(3, A, Some(3)).unwrap();
    ws.apply(A, vec![desc("a", "kept")]).unwrap();
    ws.write(
        "brickie.toml",
        &ROOT.replace("name = \"b\"", "name = \"b\"\ndesc = \"hand\""),
    );
    let d = ws.design();
    let e = d.publish(5, A, None).unwrap_err();
    let stale = e.downcast_ref::<Stale>().expect("a stale publish");
    assert!(
        stale.message.contains("forked from v1"),
        "{}",
        stale.message
    );
    assert!(
        stale.message.contains("brickie design rebase --base 5"),
        "{}",
        stale.message
    );
    let out = d.rebase(5, A).unwrap().version;
    assert_eq!(
        (out.v, out.forked_from, out.track),
        (7, Some(6), Track::Draft)
    );
    let text = ws.read("brickie.toml");
    for want in ["desc = \"hand\"", "name = \"c\"", "desc = \"kept\""] {
        assert!(text.contains(want), "{want} in {text}");
    }
    assert!(
        !text.contains("desc = \"x\""),
        "an undone op is not replayed: {text}"
    );
    assert!(
        d.rebase(7, A).is_ok_and(|o| !o.changed),
        "nothing to rebase"
    );
}

#[test]
fn rebase_fails_on_a_clash_and_writes_nothing() {
    let ws = Ws::new();
    ws.apply(A, vec![desc("b", "x")]).unwrap();
    let before = ws.read("brickie.toml");
    let root = ROOT.replace("[[brick]]\nname = \"b\"\nmembers = [\"pb\"]\n", "");
    let root = root.replace(
        "[[rule]]\nfrom = \"a\"\nto = \"b\"\naction = \"allow\"\n",
        "",
    );
    ws.write("brickie.toml", &root);
    let e = format!("{:#}", ws.design().rebase(2, A).unwrap_err());
    assert!(e.contains("clashes with the published changes"), "{e}");
    assert_eq!(ws.read("brickie.toml"), before);
    assert_eq!(ws.design().forked_from().unwrap(), Some(1));
}
