use super::*;

fn node(id: &str, path: Option<&str>, children: Vec<Node>) -> Node {
    Node {
        id: id.into(),
        label: id.into(),
        path: path.map(Into::into),
        badge: None,
        children,
        kind: codeview_port::NodeKind::default(),
        tags: Vec::new(),
    }
}

fn tree() -> Vec<Node> {
    vec![node(
        "root",
        None,
        vec![
            node(
                "root/a",
                None,
                vec![node("x", Some("crates/a/src/lib.rs"), vec![])],
            ),
            node(
                "root/b",
                None,
                vec![node("y", Some("crates/b/src/main.rs"), vec![])],
            ),
        ],
    )]
}

#[test]
fn query_params_decode() {
    let q = Query::parse("?path=crates%2Fa%20b.rs&line=12&col=5&filter=root/web&x=1");
    assert_eq!(q.path.as_deref(), Some("crates/a b.rs"));
    assert_eq!(q.line, Some(12));
    assert_eq!(q.col, Some(5));
    assert_eq!(q.filter.as_deref(), Some("root/web"));
    assert_eq!(q.view, None);
    assert_eq!(Query::parse(""), Query::default());
    assert_eq!(Query::parse("path=%zz%").path.as_deref(), Some("%zz%"));
}

#[test]
fn filter_scopes_the_tree() {
    let t = tree();
    assert_eq!(scoped(&t, Some("root/b"))[0].id, "root/b");
    assert_eq!(scoped(&t, Some("nope"))[0].id, "root");
    assert_eq!(
        files(&t),
        vec!["crates/a/src/lib.rs", "crates/b/src/main.rs"]
    );
}

#[test]
fn quick_open_prefers_file_names() {
    let paths = vec![
        "crates/main/src/x.rs".to_string(),
        "crates/b/src/main.rs".to_string(),
        "README.md".to_string(),
    ];
    assert_eq!(
        quick("main", &paths, 10),
        vec!["crates/b/src/main.rs", "crates/main/src/x.rs"]
    );
    assert_eq!(quick("rdme", &paths, 10), vec!["README.md"]);
    assert_eq!(quick("", &paths, 2).len(), 2);
    assert!(quick("zzz", &paths, 10).is_empty());
}

#[test]
fn tabs_open_and_close() {
    let t = open_tab(&[], "a");
    let t = open_tab(&t, "b");
    assert_eq!(open_tab(&t, "a"), vec!["a", "b"]);
    assert_eq!(
        close_tab(&t, "a"),
        (vec!["b".to_string()], Some("b".to_string()))
    );
    assert_eq!(
        close_tab(&t, "b"),
        (vec!["a".to_string()], Some("a".to_string()))
    );
    assert_eq!(close_tab(&["a".to_string()], "a"), (vec![], None));
    assert_eq!(file_name("a/b/c.rs"), "c.rs");
}

#[test]
fn entry_file_prefers_the_root_of_a_crate_or_folder() {
    let f = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        entry_file(&f(&["a/src/x/util.rs", "a/src/x/mod.rs", "a/src/lib.rs"])).as_deref(),
        Some("a/src/lib.rs")
    );
    assert_eq!(
        entry_file(&f(&["a/src/x/util.rs", "a/src/x/mod.rs"])).as_deref(),
        Some("a/src/x/mod.rs")
    );
    assert_eq!(entry_file(&f(&["b.rs", "a.rs"])).as_deref(), Some("b.rs"));
    assert_eq!(entry_file(&[]), None);
}

#[test]
fn tag_hues_are_stable_and_spread() {
    assert_eq!(tag_hue("core"), tag_hue("core"));
    assert_ne!(tag_hue("core"), tag_hue("web"));
    assert!(tag_hue("anything") < 360);
}
