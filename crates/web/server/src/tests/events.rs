//! Server-Sent Events: the watcher's snapshots turn writes from anywhere (here the core API, as
//! the CLI would) into events, and `/api/events` streams the current state first.

use super::*;
use crate::events::snapshot;
use brickie_change::{By, Change};
use brickie_web_api::ServerEvent;

#[test]
fn writes_from_anywhere_become_events() {
    let s = Site::new();
    let st = AppState::new(s.config());
    assert!(
        st.events.update(snapshot(&st, 0)).is_empty(),
        "first look: no event"
    );
    let c = Change::open(&s.config(), "events").unwrap();
    let op = serde_json::from_value(json!({"op": "add_brick", "file": "brickie.toml",
        "brick": {"name": "cache", "kind": "crate"}}))
    .unwrap();
    c.record(By::Agent, op, None).unwrap();
    let ev = st.events.update(snapshot(&st, 0));
    assert_eq!(
        ev,
        vec![ServerEvent::Change {
            open: true,
            head: 1
        }]
    );
    // Nothing new: no event.
    assert!(st.events.update(snapshot(&st, 0)).is_empty());
    // `cache.json` replaced (as `brickie measure` does) and a toml edited.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let cache = st.cache_file();
    let mut text = std::fs::read_to_string(&cache).unwrap();
    text.push('\n');
    std::fs::write(&cache, text).unwrap();
    let ev = st.events.update(snapshot(&st, 7));
    assert!(matches!(ev[0], ServerEvent::Cache { .. }), "{ev:?}");
    assert_eq!(ev[1], ServerEvent::Config { stamp: 7 });
}

#[tokio::test]
async fn the_stream_starts_with_the_current_state() {
    let s = Site::new();
    let req = Request::get("/api/events").body(Body::empty()).unwrap();
    let resp = router(AppState::new(s.config()))
        .oneshot(req)
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers()["content-type"].to_str().unwrap().to_string();
    assert!(ct.starts_with("text/event-stream"), "{ct}");
    let mut body = resp.into_body();
    let frame = body.frame().await.unwrap().unwrap().into_data().unwrap();
    let text = String::from_utf8_lossy(&frame).to_string();
    assert!(text.contains("event: change"), "{text}");
    assert!(text.contains(r#""open":false"#), "{text}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_watcher_pushes_what_changed() {
    let s = Site::new();
    let st = AppState::new(s.config());
    let mut rx = st.events.subscribe();
    crate::events::spawn(st.clone());
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert!(st.events.last().is_some(), "the watcher looked");
    Change::open(&s.config(), "watched").unwrap();
    let ev = tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv())
        .await
        .expect("an event within 3 s")
        .unwrap();
    assert_eq!(
        ev,
        ServerEvent::Change {
            open: true,
            head: 0
        }
    );
}

#[test]
fn a_snapshot_lists_and_diffs_its_state() {
    use crate::events::scan::tree_stamp;
    use crate::events::{Hub, Snapshot};
    let s = Site::new();
    let st = AppState::new(s.config());
    let tree = tree_stamp(s.config().parent().unwrap());
    assert_ne!(tree, 0, "the tree has a brickie.toml");
    let a: Snapshot = snapshot(&st, tree);
    assert_eq!(a.events().len(), 4);
    assert!(a.diff(&a).is_empty());
    assert!(Hub::new().update(a).is_empty());
}
