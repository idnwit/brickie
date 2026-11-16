//! The `independent` brick key survives every edit path (documents are edited in place).

use super::*;

#[test]
fn independent_key_survives_edits() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        &ROOT.replace(
            "members = [\"pb\"]",
            "members = [\"pb\"]\nindependent = true",
        ),
    );
    ws.apply(
        H,
        vec![
            desc("b", "a library"),
            Op::RenameBrick {
                file: "brickie.toml".into(),
                from: "b".into(),
                to: "lib".into(),
            },
            add("c", &["pc"]),
        ],
    )
    .unwrap();
    let now = ws.read("brickie.toml");
    assert!(now.contains("independent = true"), "{now}");
    assert!(now.contains("name = \"lib\""), "{now}");
    // Undo restores bytes too.
    ws.design().undo(2, H, None).unwrap();
    assert!(ws.read("brickie.toml").contains("independent = true"));
}
