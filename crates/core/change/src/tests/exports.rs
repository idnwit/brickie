//! `add_export` and `remove_export` edit the current exports list one entry at a time.

use super::*;

const CHILD: &str = r#"[[brick]]
name = "x"
members = ["path:x/"]

[[brick]]
name = "y"
members = ["py"]

[[brick]]
name = "z"
members = ["pz"]
"#;

fn add(brick: &str, scope: Option<&str>) -> Op {
    Op::AddExport {
        file: "g/brickie.toml".into(),
        brick: brick.into(),
        scope: scope.map(String::from),
    }
}

fn rm(brick: &str) -> Op {
    Op::RemoveExport {
        file: "g/brickie.toml".into(),
        brick: brick.into(),
    }
}

fn ws() -> Ws {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        &ROOT.replace("tags = [\"core\"]", "path = \"g\""),
    );
    ws.write("g/brickie.toml", CHILD);
    ws
}

#[test]
fn several_exports_accumulate_in_one_batch() {
    let ws = ws();
    let ops = vec![add("x", None), add("y", Some("prod")), add("z", None)];
    ws.apply(H, ops).unwrap();
    let text = ws.read("g/brickie.toml");
    assert!(text.contains("\"x\"") && text.contains("\"z\""), "{text}");
    assert!(text.contains("brick = \"y\"") && text.contains("scope = \"prod\""));
}

#[test]
fn add_replaces_the_scope_and_remove_keeps_the_rest() {
    let ws = ws();
    ws.apply(H, vec![add("x", None), add("y", None)]).unwrap();
    ws.apply(H, vec![add("x", Some("test")), rm("y"), rm("z")])
        .unwrap();
    let text = ws.read("g/brickie.toml");
    assert!(text.contains("brick = \"x\"") && text.contains("scope = \"test\""));
    assert!(!text.contains("\"y\"") || text.contains("name = \"y\""));
    assert_eq!(text.matches("brick = \"x\"").count(), 1);
}
