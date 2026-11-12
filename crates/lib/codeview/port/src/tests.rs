use super::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

fn round<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(v: T) -> String {
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(serde_json::from_str::<T>(&json).unwrap(), v);
    json
}

fn span() -> Span {
    Span {
        start: Pos { line: 3, col: 5 },
        end: Pos { line: 3, col: 9 },
    }
}

#[test]
fn data_round_trips() {
    round(View {
        name: "Bricks".into(),
        roots: vec![Node {
            id: "root/web".into(),
            label: "web".into(),
            path: None,
            badge: Some(Badge {
                errors: 1,
                warns: 2,
            }),
            kind: NodeKind::Composite,
            tags: vec!["core".into()],
            children: vec![Node {
                id: "a.rs".into(),
                label: "a.rs".into(),
                path: Some("a.rs".into()),
                badge: None,
                children: vec![],
                kind: NodeKind::File,
                tags: vec![],
            }],
        }],
    });
    round(FileText {
        path: "a.rs".into(),
        text: "fn main() {}".into(),
        lang: "rust".into(),
        stale: true,
    });
    let j = round(Mark {
        span: span(),
        level: Level::Warn,
        message: "m".into(),
        link: Some("/?item=x".into()),
    });
    assert!(j.contains("\"level\":\"warn\""), "{j}");
    round(Band {
        start_line: 2,
        end_line: 9,
        depth: 1,
        heat: 0.5,
        label: "if".into(),
        detail: "nesting 1".into(),
    });
    round(Location {
        path: "a.rs".into(),
        span: span(),
        label: "root/web".into(),
    });
    round(AtReq {
        path: "a.rs".into(),
        pos: Pos { line: 1, col: 2 },
    });
}

#[test]
fn messages_are_tagged() {
    let j = round(ToViewer::Open {
        path: "a.rs".into(),
        span: Some(span()),
    });
    assert!(j.starts_with("{\"type\":\"Open\""), "{j}");
    round(ToViewer::Filter {
        view: "Bricks".into(),
        node: None,
    });
    round(ToViewer::Theme { dark: true });
    assert_eq!(round(ToHost::Close), "{\"type\":\"Close\"}");
    round(ToHost::Opened { path: "a".into() });
    assert_eq!(round(ToHost::Ready), "{\"type\":\"Ready\"}");
    round(ToHost::Link { href: "/".into() });
    let m: ToViewer = serde_json::from_str(r#"{"type":"Open","path":"x.rs","span":null}"#).unwrap();
    assert_eq!(
        m,
        ToViewer::Open {
            path: "x.rs".into(),
            span: None
        }
    );
}

#[test]
fn paths_are_confined() {
    assert!(valid_path("crates/a/src/lib.rs"));
    for bad in [
        "",
        "/etc/passwd",
        "../x",
        "a/../b",
        "a//b",
        "./a",
        "a\\b",
        "c:x",
        "a\0",
    ] {
        assert!(!valid_path(bad), "{bad}");
    }
}

#[test]
fn nodes_without_presentation_fields_still_parse() {
    let n: Node =
        serde_json::from_str(r#"{"id":"a","label":"a","path":null,"badge":null,"children":[]}"#)
            .unwrap();
    assert_eq!((n.kind, n.tags.len()), (NodeKind::Group, 0));
}
