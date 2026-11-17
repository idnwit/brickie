//! Validation of one level: rule selectors, links and exports name this file's own bricks and tags,
//! at any level. Every failure is `config/invalid`.

use super::{Config, KINDS, Sel};
use anyhow::{Result, bail};
use brickie_model::codes::CONFIG_INVALID;
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
        kinds(&r.kinds, &r.loc)?;
        if r.reason.trim().is_empty() {
            bail!("{CONFIG_INVALID}: {}: a rule needs a reason", r.loc);
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
    Ok(())
}

/// Each link names a brick of this level as `from`, and as `to` a sibling or
/// `composite/child` where the child is exported by that composite. Runs once the composites
/// below `cfg` are loaded, because it reads their `exports`.
pub fn validate_links(cfg: &Config) -> Result<()> {
    for l in &cfg.links {
        let bad = |why: String| bail!("{CONFIG_INVALID}: {}: {why}", l.loc);
        let (to, child) = l.target();
        if cfg.brick(&l.from).is_none() {
            return bad(format!("unknown brick '{}'", l.from));
        }
        let Some(target) = cfg.brick(to) else {
            return bad(format!("unknown brick '{to}'"));
        };
        if to == l.from {
            return bad(format!("link from '{to}' to itself"));
        }
        kinds(&l.kinds, &l.loc)?;
        if l.reason.trim().is_empty() {
            return bad("a link needs a reason".into());
        }
        if let Some(why) = child.and_then(|c| child_error(target, c)) {
            return bad(why);
        }
    }
    Ok(())
}

/// Why `child` of `target` cannot be the end of a link, if it cannot.
fn child_error(target: &super::Brick, child: &str) -> Option<String> {
    let to = &target.name;
    let Some(sub) = target.sub.as_deref() else {
        return Some(format!("'{to}' is not a composite, so it names no child"));
    };
    if sub.brick(child).is_none() || child.contains('/') {
        return Some(format!("'{child}' is not a child of '{to}'"));
    }
    (!sub.exports(child)).then(|| format!("'{to}' does not export '{child}'"))
}

fn kinds(kinds: &[String], loc: &str) -> Result<()> {
    if let Some(k) = kinds.iter().find(|k| !KINDS.contains(&k.as_str())) {
        bail!("{CONFIG_INVALID}: {loc}: unknown kind '{k}' (expected one of {KINDS:?})");
    }
    Ok(())
}
