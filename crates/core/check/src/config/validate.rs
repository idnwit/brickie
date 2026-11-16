//! Validation of one level: rule selectors and exports name this file's own bricks and tags,
//! and `independent` is root-only. Every failure is `config/invalid`.

use super::{Config, KINDS, Sel};
use anyhow::{Result, bail};
use brickie_model::codes::{CONFIG_INVALID, CONFIG_UNKNOWN_VIA};
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
            Sel::Tag(t) if !tags.contains(t.as_str()) => {
                bail!("{CONFIG_INVALID}: {loc}: unknown tag '{t}'")
            }
            Sel::Brick(n) if !names.contains(n.as_str()) => {
                bail!("{CONFIG_INVALID}: {loc}: unknown brick '{n}'")
            }
            _ => Ok(()),
        }
    };
    for r in &cfg.rules {
        check(&r.from, &r.loc)?;
        check(&r.to, &r.loc)?;
        if let Some(k) = r.kinds.iter().find(|k| !KINDS.contains(&k.as_str())) {
            bail!(
                "{CONFIG_INVALID}: {}: unknown kind '{k}' (expected one of {KINDS:?})",
                r.loc
            );
        }
    }
    for r in &cfg.complexity {
        check(&r.bricks, &r.loc)?;
    }
    if let Some(e) = cfg.exports.iter().find(|e| !names.contains(e.as_str())) {
        bail!(
            "{CONFIG_INVALID}: {} exports '{e}', which is not a brick of that file",
            cfg.file
        );
    }
    if let Some(b) = cfg
        .bricks
        .iter()
        .find(|b| b.independent && !cfg.path.is_empty())
    {
        bail!(
            "{CONFIG_INVALID}: brick '{}' in {} is independent, which only a root brick may be",
            b.name,
            cfg.file
        );
    }
    Ok(())
}

/// `via` names exported children of a composite `to` brick. Runs once the composites below
/// `cfg` are loaded, because it reads their `exports`.
pub fn validate_via(cfg: &Config) -> Result<()> {
    for r in &cfg.rules {
        let Some(via) = &r.via else { continue };
        let bad = |why: String| bail!("{CONFIG_UNKNOWN_VIA}: {}: {why}", r.loc);
        let target = match &r.to {
            Sel::Brick(n) => cfg.brick(n),
            _ => None,
        };
        let Some(sub) = target.and_then(|b| b.sub.as_deref()) else {
            return bad(format!(
                "`via` needs `to` to name a composite brick, not '{}'",
                r.to
            ));
        };
        if via.is_empty() {
            return bad("`via` is an empty list".into());
        }
        if let Some(v) = via.iter().find(|v| !sub.exports(v)) {
            return bad(format!("'{v}' is not exported by '{}'", r.to));
        }
    }
    Ok(())
}
