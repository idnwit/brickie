//! Validation of one level: rule selectors and exports name this file's own bricks and tags.

use super::{Config, KINDS, Sel};
use anyhow::{Result, bail};
use brickie_model::codes::CONFIG_UNKNOWN_EXPORT;
use std::collections::BTreeSet;

pub fn validate(cfg: &Config) -> Result<()> {
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
    for e in &cfg.exports {
        if !names.contains(e.brick.as_str()) {
            bail!(
                "{CONFIG_UNKNOWN_EXPORT}: {} exports '{}', which is not a brick of that file",
                cfg.file,
                e.brick
            );
        }
    }
    Ok(())
}
