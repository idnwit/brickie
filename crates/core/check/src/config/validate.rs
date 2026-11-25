//! Validation of one level: rule selectors, links, groups and exports name this file's own
//! bricks, groups and tags, at any level. Every failure is `config/invalid`.

use super::{Config, KINDS, Sel};
use anyhow::{Result, bail};
use brickie_model::codes::CONFIG_INVALID;
use std::collections::BTreeSet;

fn invalid_group_name(name: &str) -> bool {
    name.is_empty()
        || name.chars().any(char::is_whitespace)
        || name.contains('/')
        || name.contains('*')
        || name == "root"
        || name.contains("tag:")
}

fn grouped_target_error(cfg: &Config, name: &str) -> Option<String> {
    if let Some(g) = cfg.brick(name).and_then(|b| b.group.as_deref()) {
        return Some(format!(
            "brick '{name}' is in group '{g}'; target the group name '{g}' instead"
        ));
    }
    if let Some((g, _)) = name.split_once('/')
        && cfg.group(g).is_some()
    {
        return Some(format!(
            "'{name}' names a member of group '{g}'; target the group name '{g}' instead"
        ));
    }
    None
}

pub fn validate(cfg: &Config) -> Result<()> {
    let group_names = validate_groups(cfg)?;
    validate_bricks(cfg, &group_names)?;
    let tags = all_tags(cfg);
    validate_rules_and_limits(cfg, &tags)?;
    validate_exports(cfg)
}

fn validate_groups(cfg: &Config) -> Result<BTreeSet<&str>> {
    let brick_names: BTreeSet<&str> = cfg.bricks.iter().map(|b| b.name.as_str()).collect();
    let mut group_names: BTreeSet<&str> = BTreeSet::new();
    for g in &cfg.groups {
        if invalid_group_name(&g.name) {
            bail!(
                "{CONFIG_INVALID}: {}: invalid group name '{}'",
                g.loc,
                g.name
            );
        }
        if brick_names.contains(g.name.as_str()) || !group_names.insert(g.name.as_str()) {
            bail!("{CONFIG_INVALID}: {}: duplicate name '{}'", g.loc, g.name);
        }
    }
    Ok(group_names)
}

fn validate_bricks(cfg: &Config, group_names: &BTreeSet<&str>) -> Result<()> {
    for b in &cfg.bricks {
        let Some(g) = &b.group else {
            continue;
        };
        if !group_names.contains(g.as_str()) {
            bail!(
                "{CONFIG_INVALID}: {}: brick '{}' references unknown group '{g}'",
                cfg.file,
                b.name
            );
        }
        if !b.tags.is_empty() {
            bail!(
                "{CONFIG_INVALID}: {}: brick '{}' is in group '{g}': tags must be set on the group",
                cfg.file,
                b.name
            );
        }
        if b.independent {
            bail!(
                "{CONFIG_INVALID}: {}: brick '{}' is in group '{g}': grouped bricks cannot be independent",
                cfg.file,
                b.name
            );
        }
    }
    Ok(())
}

fn all_tags(cfg: &Config) -> BTreeSet<&str> {
    cfg.bricks
        .iter()
        .flat_map(|b| b.tags.iter().map(String::as_str))
        .chain(
            cfg.groups
                .iter()
                .flat_map(|g| g.tags.iter().map(String::as_str)),
        )
        .collect()
}

fn validate_selector(cfg: &Config, tags: &BTreeSet<&str>, sel: &Sel, loc: &str) -> Result<()> {
    match sel {
        Sel::Tag(t) if !tags.contains(t.as_str()) => {
            bail!("{CONFIG_INVALID}: {loc}: unknown tag '{t}'")
        }
        Sel::Brick(n) => {
            if let Some(why) = grouped_target_error(cfg, n) {
                bail!("{CONFIG_INVALID}: {loc}: {why}");
            }
            if cfg.unit_by_name(n).is_none() {
                bail!("{CONFIG_INVALID}: {loc}: unknown brick '{n}'");
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_rules_and_limits(cfg: &Config, tags: &BTreeSet<&str>) -> Result<()> {
    for r in &cfg.rules {
        validate_selector(cfg, tags, &r.from, &r.loc)?;
        validate_selector(cfg, tags, &r.to, &r.loc)?;
        kinds(&r.kinds, &r.loc)?;
        if r.reason.trim().is_empty() {
            bail!("{CONFIG_INVALID}: {}: a rule needs a reason", r.loc);
        }
    }
    for r in &cfg.complexity {
        validate_selector(cfg, tags, &r.bricks, &r.loc)?;
    }
    Ok(())
}

fn validate_exports(cfg: &Config) -> Result<()> {
    for e in &cfg.exports {
        if let Some(why) = grouped_target_error(cfg, e) {
            bail!("{CONFIG_INVALID}: {}: {why}", cfg.file);
        }
        if cfg.unit_by_name(e).is_none() {
            bail!(
                "{CONFIG_INVALID}: {} exports '{e}', which is not a brick or group of that file",
                cfg.file
            );
        }
    }
    Ok(())
}

/// Each link names a unit (brick or group) of this level as `from`, and as `to` a sibling or
/// `composite/child` where the child (brick or group) is exported by that composite. Runs once
/// the composites below `cfg` are loaded, because it reads their `exports`.
pub fn validate_links(cfg: &Config) -> Result<()> {
    for l in &cfg.links {
        validate_link(cfg, l)?;
    }
    Ok(())
}

fn validate_link(cfg: &Config, l: &super::Link) -> Result<()> {
    let (to, child) = l.target();
    if let Some(why) = grouped_target_error(cfg, &l.from) {
        bail!("{CONFIG_INVALID}: {}: {why}", l.loc);
    }
    if cfg.unit_by_name(&l.from).is_none() {
        bail!("{CONFIG_INVALID}: {}: unknown brick '{}'", l.loc, l.from);
    }
    let target_brick = validate_link_to(cfg, l, to, child)?;
    if to == l.from {
        bail!("{CONFIG_INVALID}: {}: link from '{to}' to itself", l.loc);
    }
    kinds(&l.kinds, &l.loc)?;
    if l.reason.trim().is_empty() {
        bail!("{CONFIG_INVALID}: {}: a link needs a reason", l.loc);
    }
    if let Some(target) = target_brick
        && let Some(why) = child.and_then(|c| child_error(target, c))
    {
        bail!("{CONFIG_INVALID}: {}: {why}", l.loc);
    }
    Ok(())
}

fn validate_link_to<'a>(
    cfg: &'a Config,
    l: &super::Link,
    to: &str,
    child: Option<&str>,
) -> Result<Option<&'a super::Brick>> {
    if let Some(why) = grouped_target_error(cfg, to) {
        bail!("{CONFIG_INVALID}: {}: {why}", l.loc);
    }
    if cfg.group(to).is_some() {
        if child.is_some() {
            bail!(
                "{CONFIG_INVALID}: {}: '{}' names a member of group '{to}'; target the group name '{to}' instead",
                l.loc,
                l.to
            );
        }
        return Ok(None);
    }
    if let Some(b) = cfg.brick(to) {
        return Ok(Some(b));
    }
    bail!("{CONFIG_INVALID}: {}: unknown brick '{to}'", l.loc)
}

/// Why `child` of `target` cannot be the end of a link, if it cannot.
fn child_error(target: &super::Brick, child: &str) -> Option<String> {
    let to = &target.name;
    let Some(sub) = target.sub.as_deref() else {
        return Some(format!("'{to}' is not a composite, so it names no child"));
    };
    if child.contains('/') {
        return Some(format!("'{child}' is not a child of '{to}'"));
    }
    if let Some(g) = sub.brick(child).and_then(|b| b.group.as_deref()) {
        return Some(format!(
            "'{child}' is in group '{g}' of '{to}'; target the group name '{g}' instead"
        ));
    }
    if sub.unit_by_name(child).is_none() {
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
