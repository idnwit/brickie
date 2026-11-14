//! `via` entries in a parent rule follow the child bricks of the composite they name.

use super::*;

const CHILD: &str = r#"exports = ["ports-in", "ports-out"]

[[brick]]
name = "ports-in"
members = ["pi"]

[[brick]]
name = "ports-out"
members = ["po"]
"#;

fn ws() -> Ws {
    let ws = Ws::new();
    let root = ROOT
        .replace("tags = [\"core\"]", "path = \"g\"")
        .replace("to = \"b\"", "to = \"a\"\nvia = [\"ports-in\"]")
        .replace("from = \"a\"", "from = \"b\"");
    ws.write("brickie.toml", &root);
    ws.write("g/brickie.toml", CHILD);
    ws
}

#[test]
fn child_rename_rewrites_the_parent_via() {
    let ws = ws();
    let rename = Op::RenameBrick {
        file: "g/brickie.toml".into(),
        from: "ports-in".into(),
        to: "inbound".into(),
    };
    ws.apply(A, vec![rename]).unwrap();
    assert!(ws.read("brickie.toml").contains("via = [\"inbound\"]"));
    ws.design().config().unwrap();
}

#[test]
fn composite_rename_keeps_via() {
    let ws = ws();
    let rename = Op::RenameBrick {
        file: "brickie.toml".into(),
        from: "a".into(),
        to: "app".into(),
    };
    ws.apply(A, vec![rename]).unwrap();
    let text = ws.read("brickie.toml");
    assert!(text.contains("to = \"app\"") && text.contains("via = [\"ports-in\"]"));
}

#[test]
fn removing_or_unexporting_a_via_child_is_rejected() {
    let ws = ws();
    let remove = Op::RemoveBrick {
        file: "g/brickie.toml".into(),
        name: "ports-in".into(),
    };
    let e = format!("{:#}", ws.apply(A, vec![remove]).unwrap_err());
    assert!(e.contains("design/invalid") && e.contains("rule 0"), "{e}");
    let unexport = Op::SetExports {
        file: "g/brickie.toml".into(),
        exports: vec![crate::ExportSpec {
            brick: "ports-out".into(),
            scope: None,
        }],
    };
    assert!(ws.apply(A, vec![unexport]).is_err());
    let ungroup = Op::Ungroup {
        file: "brickie.toml".into(),
        name: "a".into(),
    };
    assert!(ws.apply(A, vec![ungroup]).is_err());
    assert_eq!(ws.read("g/brickie.toml"), CHILD);
    // Another child is free to go, with its export.
    let ok = Op::RemoveBrick {
        file: "g/brickie.toml".into(),
        name: "ports-out".into(),
    };
    let exports = Op::SetExports {
        file: "g/brickie.toml".into(),
        exports: vec![crate::ExportSpec {
            brick: "ports-in".into(),
            scope: None,
        }],
    };
    // Alone it leaves an unknown export: the whole design is validated before any write.
    assert!(ws.apply(A, vec![ok.clone()]).is_err());
    ws.apply(A, vec![exports, ok]).unwrap();
}
