//! Loading `brickie.toml` with `include` merging, and validation.

use super::{Action, Brick, ComplexityRule, Config, DepRule, KINDS, Sel};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawDeps {
    default: Option<Action>,
    cycles: Option<Action>,
    #[serde(default)]
    rule: Vec<DepRule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    brick: Vec<Brick>,
    #[serde(default)]
    deps: RawDeps,
    #[serde(default)]
    complexity: Vec<ComplexityRule>,
}

/// Line numbers (1-based) of the `[[header]]` table starts, in file order.
fn header_lines(text: &str, header: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| l.trim() == header)
        .map(|(i, _)| i + 1)
        .collect()
}

pub fn load(path: &Path) -> Result<Config> {
    let mut cfg = Config::default();
    let mut stack = Vec::new();
    let mut done = BTreeSet::new();
    load_file(path, &mut cfg, &mut stack, &mut done)?;
    validate(&cfg)?;
    Ok(cfg)
}

/// Includes are merged before the including file: lists are concatenated, and `default` /
/// `cycles` set by the including file override those of its includes.
fn load_file(
    path: &Path,
    cfg: &mut Config,
    stack: &mut Vec<PathBuf>,
    done: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let canon = path
        .canonicalize()
        .with_context(|| format!("read {}", path.display()))?;
    if stack.contains(&canon) {
        bail!("include cycle: {} includes itself", path.display());
    }
    if !done.insert(canon.clone()) {
        return Ok(()); // already merged through another include
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let raw: Raw = toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    stack.push(canon);
    let dir = path.parent().unwrap_or(Path::new("."));
    for inc in &raw.include {
        load_file(&dir.join(inc), cfg, stack, done)
            .with_context(|| format!("included from {}", path.display()))?;
    }
    stack.pop();
    for b in raw.brick {
        if cfg.bricks.iter().any(|x| x.name == b.name) {
            bail!("duplicate brick '{}' (in {})", b.name, path.display());
        }
        cfg.bricks.push(b);
    }
    if let Some(d) = raw.deps.default {
        cfg.deps.default = d;
    }
    if let Some(c) = raw.deps.cycles {
        cfg.deps.cycles = c;
    }
    let loc = |lines: &[usize], i: usize| {
        format!("{}:{}", path.display(), lines.get(i).copied().unwrap_or(0))
    };
    let lines = header_lines(&text, "[[deps.rule]]");
    for (i, mut r) in raw.deps.rule.into_iter().enumerate() {
        r.loc = loc(&lines, i);
        cfg.deps.rules.push(r);
    }
    let lines = header_lines(&text, "[[complexity]]");
    for (i, mut r) in raw.complexity.into_iter().enumerate() {
        r.loc = loc(&lines, i);
        cfg.complexity.push(r);
    }
    Ok(())
}

fn validate(cfg: &Config) -> Result<()> {
    let tags: BTreeSet<&str> = cfg
        .bricks
        .iter()
        .flat_map(|b| b.tags.iter().map(String::as_str))
        .collect();
    let names: BTreeSet<&str> = cfg.bricks.iter().map(|b| b.name.as_str()).collect();
    let check = |sel: &Sel, loc: &str| -> Result<()> {
        match sel {
            Sel::Tag(t) if !tags.contains(t.as_str()) => bail!("{loc}: unknown tag '{t}'"),
            Sel::Brick(n) if !names.contains(n.as_str()) => bail!("{loc}: unknown brick '{n}'"),
            _ => Ok(()),
        }
    };
    for r in &cfg.deps.rules {
        check(&r.from, &r.loc)?;
        check(&r.to, &r.loc)?;
        if let Some(k) = r.kinds.iter().find(|k| !KINDS.contains(&k.as_str())) {
            bail!("{}: unknown kind '{k}' (expected one of {KINDS:?})", r.loc);
        }
    }
    for r in &cfg.complexity {
        check(&r.bricks, &r.loc)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_str(text: &str) -> Result<Config> {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("brickie.toml");
        std::fs::write(&p, text).unwrap();
        load(&p)
    }

    #[test]
    fn parses_and_rejects() {
        let c =
            load_str("[deps]\ndefault=\"deny\"\n[[brick]]\nname=\"a\"\nmembers=[\"x\"]\n").unwrap();
        assert_eq!(c.bricks.len(), 1);
        assert_eq!(c.deps.default, Action::Deny);
        let dup = "[[brick]]\nname=\"a\"\nmembers=[]\n[[brick]]\nname=\"a\"\nmembers=[]\n";
        assert!(
            load_str(dup)
                .unwrap_err()
                .to_string()
                .contains("duplicate brick")
        );
        assert!(load_str("bogus = 1\n").is_err());
    }
}
