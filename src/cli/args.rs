//! Minimal argument parsing: positionals and `--name value` flags.

use anyhow::{Result, bail};

pub const USAGE: &str = "usage:
  brickie measure [--config brickie.toml] [--scip FILE] [--workspace DIR]
  brickie check [--config brickie.toml] [--format text|json] [--color auto|always|never]
  brickie view [--config brickie.toml] [--port 7881] [--bind 127.0.0.1]
  brickie llms.txt";

pub struct Args {
    pub positional: Vec<String>,
    flags: Vec<(String, String)>,
}

pub fn parse(args: impl Iterator<Item = String>) -> Result<Args> {
    let mut a = Args {
        positional: vec![],
        flags: vec![],
    };
    let mut it = args;
    while let Some(x) = it.next() {
        if let Some(name) = x.strip_prefix("--") {
            let Some(v) = it.next() else {
                bail!("--{name} needs a value\n{USAGE}");
            };
            a.flags.push((name.to_string(), v));
        } else {
            a.positional.push(x);
        }
    }
    Ok(a)
}

impl Args {
    pub fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}
