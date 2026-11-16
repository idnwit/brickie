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

fn add(brick: &str) -> Op {
    Op::AddExport {
        file: "g/brickie.toml".into(),
        brick: brick.into(),
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
    ws.apply(H, vec![add("x"), add("y"), add("z")]).unwrap();
    let text = ws.read("g/brickie.toml");
    assert!(text.contains("exports = [\"x\", \"y\", \"z\"]"), "{text}");
}

#[test]
fn add_is_idempotent_and_remove_keeps_the_rest() {
    let ws = ws();
    ws.apply(H, vec![add("x"), add("y")]).unwrap();
    ws.apply(H, vec![add("x"), rm("y"), rm("z")]).unwrap();
    let text = ws.read("g/brickie.toml");
    assert!(text.contains("exports = [\"x\"]"), "{text}");
}
