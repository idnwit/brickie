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

fn set_independent(file: &str, name: &str, on: bool) -> Op {
    Op::SetBrick {
        file: file.into(),
        name: name.into(),
        desc: None,
        tags: None,
        members: None,
        path: None,
        independent: Some(on),
        group: None,
    }
}

#[test]
fn set_brick_sets_and_unsets_independent() {
    let ws = Ws::new();
    ws.apply(H, vec![set_independent("brickie.toml", "b", true)])
        .unwrap();
    assert!(ws.read("brickie.toml").contains("independent = true"));
    ws.apply(H, vec![set_independent("brickie.toml", "b", false)])
        .unwrap();
    assert!(!ws.read("brickie.toml").contains("independent"));
}

#[test]
fn independent_on_a_nested_brick_is_allowed() {
    let ws = Ws::new();
    ws.apply(H, vec![add("g", &[])]).unwrap();
    ws.apply(
        H,
        vec![
            Op::MakeComposite {
                file: "brickie.toml".into(),
                name: "g".into(),
                path: "g".into(),
            },
            Op::AddBrick {
                file: "g/brickie.toml".into(),
                brick: BrickSpec {
                    name: "n".into(),
                    members: vec!["pn".into()],
                    ..BrickSpec::default()
                },
            },
        ],
    )
    .unwrap();
    ws.apply(H, vec![set_independent("g/brickie.toml", "n", true)])
        .unwrap();
    assert!(ws.read("g/brickie.toml").contains("independent = true"));
}
