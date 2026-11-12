//! Resolving every module to a leaf path in the brick tree, and the partition checks.
//!
//! At each level the most specific member wins (`path:` < package < module, longer wins);
//! equal specificity in two bricks is `config/overlap`. Below a composite every module must
//! land in exactly one child (`config/unassigned`), and a child's members must not reach
//! modules outside the composite (`config/member-outside-parent`). At the root an unmatched
//! module is merely unmapped.

use super::Config;
use super::member::{Spec, Subject, matches};
use anyhow::{Result, bail};
use brickie_facts::{Crate, Module};
use brickie_model::codes::{CONFIG_MEMBER_OUTSIDE_PARENT, CONFIG_OVERLAP, CONFIG_UNASSIGNED};
use std::collections::BTreeMap;
use std::path::Path;

/// Module id -> path of brick names from the root to its leaf (`["web", "api"]`). Unmapped
/// modules are absent.
#[derive(Debug, Default, Clone)]
pub struct Resolved {
    pub paths: BTreeMap<String, Vec<String>>,
}

impl Resolved {
    /// The leaf path of a module.
    pub fn path(&self, module: &str) -> Option<&[String]> {
        self.paths.get(module).map(Vec::as_slice)
    }
}

pub fn resolve(
    cfg: &Config,
    crates: &[Crate],
    modules: &[Module],
    ws_root: &Path,
) -> Result<Resolved> {
    let by_id: BTreeMap<&str, &Crate> = crates.iter().map(|c| (c.id.as_str(), c)).collect();
    let subjects: Vec<Subject> = modules
        .iter()
        .map(|m| Subject::new(m, by_id.get(m.krate.as_str()).copied(), ws_root))
        .collect();
    let all: Vec<&Subject> = subjects.iter().collect();
    let mut out = Resolved::default();
    assign(cfg, &all, &all, &mut out)?;
    Ok(out)
}

/// The best brick of `cfg` for `s`, or an overlap error.
fn best<'c>(cfg: &'c Config, s: &Subject) -> Result<Option<&'c str>> {
    let mut best: Option<(Spec, &str)> = None;
    for b in &cfg.bricks {
        for member in &b.members {
            let Some(spec) = matches(member, &b.base, s) else {
                continue;
            };
            match best {
                Some((bs, n)) if bs > spec || (bs == spec && n == b.name) => {}
                Some((bs, n)) if bs == spec => bail!(
                    "{CONFIG_OVERLAP}: module {} matches bricks '{n}' and '{}' with equal specificity (level {}, {})",
                    s.module.id,
                    b.name,
                    cfg.level(),
                    cfg.file
                ),
                _ => best = Some((spec, b.name.as_str())),
            }
        }
    }
    Ok(best.map(|(_, n)| n))
}

/// Assign `mods` (the modules of this level) to the children of `cfg`, recursively.
fn assign(cfg: &Config, all: &[&Subject], mods: &[&Subject], out: &mut Resolved) -> Result<()> {
    let root = cfg.path.is_empty();
    if !root {
        outside_parent(cfg, all, mods)?;
    }
    let mut groups: BTreeMap<&str, Vec<&Subject>> = BTreeMap::new();
    for s in mods {
        match best(cfg, s)? {
            Some(n) => groups.entry(n).or_default().push(s),
            None if root => {}
            None => bail!(
                "{CONFIG_UNASSIGNED}: module {} is in composite '{}' but in none of its bricks (level {}, {})",
                s.module.id,
                cfg.path.join("/"),
                cfg.level(),
                cfg.file
            ),
        }
    }
    for (name, group) in groups {
        let Some(b) = cfg.brick(name) else { continue };
        match &b.sub {
            Some(sub) => assign(sub, all, &group, out)?,
            None => {
                let mut path = cfg.path.clone();
                path.push(name.to_string());
                for s in group {
                    out.paths.insert(s.module.id.clone(), path.clone());
                }
            }
        }
    }
    Ok(())
}

/// A child member that matches a module outside this composite.
fn outside_parent(cfg: &Config, all: &[&Subject], mods: &[&Subject]) -> Result<()> {
    let inside: std::collections::BTreeSet<&str> =
        mods.iter().map(|s| s.module.id.as_str()).collect();
    for b in &cfg.bricks {
        for member in &b.members {
            if let Some(s) = all.iter().find(|s| {
                !inside.contains(s.module.id.as_str()) && matches(member, &b.base, s).is_some()
            }) {
                bail!(
                    "{CONFIG_MEMBER_OUTSIDE_PARENT}: member '{member}' of brick '{}' matches module {}, which is outside composite '{}' (level {}, {})",
                    b.name,
                    s.module.id,
                    cfg.path.join("/"),
                    cfg.level(),
                    cfg.file
                );
            }
        }
    }
    Ok(())
}
