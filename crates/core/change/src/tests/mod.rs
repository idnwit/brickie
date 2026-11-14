//! Engine tests on a temporary workspace (no cache: config-level behaviour only).

mod keys;
mod undo;
mod ungroup;
mod versions;
mod via;

use crate::{BrickSpec, By, Design, Op, Outcome};
use std::path::PathBuf;

pub const ROOT: &str = r#"# the design
exports = []

[[brick]]
name = "a"   # first
members = ["pa"]
tags = ["core"]

[[brick]]
name = "b"
members = ["pb"]

[deps]
default = "deny"

[[deps.rule]]
from = "a"
to = "b"
action = "allow"
"#;

pub struct Ws {
    pub dir: tempfile::TempDir,
}

impl Ws {
    pub fn new() -> Ws {
        let ws = Ws {
            dir: tempfile::tempdir().unwrap(),
        };
        ws.write("brickie.toml", ROOT);
        ws.write("Cargo.toml", "[workspace]\nmembers = [\"crates/pa\"]\n");
        ws
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    pub fn config(&self) -> PathBuf {
        self.path("brickie.toml")
    }

    pub fn write(&self, rel: &str, text: &str) {
        let p = self.path(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.path(rel)).unwrap()
    }

    pub fn design(&self) -> Design {
        Design::at(&self.config()).unwrap()
    }

    /// The head version number.
    pub fn head(&self) -> u64 {
        self.design().head().unwrap().v
    }

    /// Apply `ops` on the head.
    pub fn apply(&self, by: By, ops: Vec<Op>) -> anyhow::Result<Outcome> {
        let d = self.design();
        let base = d.head()?.v;
        d.apply(base, by, None, ops)
    }
}

pub fn add(name: &str, members: &[&str]) -> Op {
    Op::AddBrick {
        file: "brickie.toml".into(),
        brick: BrickSpec {
            name: name.into(),
            members: members.iter().map(|m| m.to_string()).collect(),
            ..BrickSpec::default()
        },
    }
}

pub fn desc(name: &str, d: &str) -> Op {
    Op::SetBrick {
        file: "brickie.toml".into(),
        name: name.into(),
        desc: Some(d.into()),
        tags: None,
        members: None,
        path: None,
    }
}

pub const H: By = By::Human;
pub const A: By = By::Agent;
