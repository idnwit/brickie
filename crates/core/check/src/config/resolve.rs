//! Resolving every module to a leaf path in the brick tree, and the partition checks.
//!
//! At each level the most specific member wins (`path:` < package < module prefix < exact module,
//! longer wins);
//! equal specificity in two bricks is `config/overlap`. Below a composite every module must
//! land in exactly one child (`config/unassigned`), and a child's members must not reach
//! modules outside the composite (`config/member-outside-parent`). At the root an unmatched
//! module is merely unmapped.
//!
//! [`Mode::Draft`] (the change preview) tolerates these three "not finished yet" errors: the
//! design resolves with best effort (an unassigned module stays unmapped, an overlap goes to
//! the first brick, a member reaching outside is ignored) and each becomes a [`Pending`].

use super::Config;
use super::member::{Spec, Subject, matches};
use anyhow::{Result, anyhow};
use brickie_facts::{Crate, Module};
use brickie_model::codes::{CONFIG_MEMBER_OUTSIDE_PARENT, CONFIG_OVERLAP, CONFIG_UNASSIGNED};
use std::collections::BTreeMap;
use std::path::Path;

/// Module id -> path of brick names from the root to its leaf (`["web", "api"]`). Unmapped
/// modules are absent.
#[derive(Debug, Default, Clone)]
pub struct Resolved {
    pub paths: BTreeMap<String, Vec<String>>,
    /// Draft mode only: the partition errors tolerated, in tree order.
    pub pending: Vec<Pending>,
}

/// How strictly the partition rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `check`: the first partition error aborts.
    Strict,
    /// A design in progress: partition errors are collected as [`Pending`].
    Draft,
}

/// A partition error a draft resolve tolerated (the design is not finished yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// `config/unassigned` | `config/overlap` | `config/member-outside-parent`.
    pub code: &'static str,
    /// The full message, as strict mode reports it (`code: ...`).
    pub message: String,
    /// The composite (level path) it happened in; `[]` is the root.
    pub level: Vec<String>,
    /// `config/unassigned`: the module left out.
    pub module: Option<String>,
}

impl Resolved {
    /// Modules no child of the composite at `level` claims (draft only).
    pub fn unassigned(&self, level: &[String]) -> Vec<&str> {
        (self.pending.iter())
            .filter(|p| p.code == CONFIG_UNASSIGNED && p.level == level)
            .filter_map(|p| p.module.as_deref())
            .collect()
    }
}

struct Ctx<'r> {
    mode: Mode,
    out: &'r mut Resolved,
}

impl Ctx<'_> {
    /// Abort (strict) or record (draft) a partition error.
    fn fail(
        &mut self,
        code: &'static str,
        cfg: &Config,
        module: Option<&str>,
        msg: String,
    ) -> Result<()> {
        let message = format!("{code}: {msg} (level {}, {})", cfg.level(), cfg.file);
        if self.mode == Mode::Strict {
            return Err(anyhow!(message));
        }
        self.out.pending.push(Pending {
            code,
            message,
            level: cfg.path.clone(),
            module: module.map(str::to_string),
        });
        Ok(())
    }
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
    mode: Mode,
) -> Result<Resolved> {
    let by_id: BTreeMap<&str, &Crate> = crates.iter().map(|c| (c.id.as_str(), c)).collect();
    let subjects: Vec<Subject> = modules
        .iter()
        .map(|m| Subject::new(m, by_id.get(m.krate.as_str()).copied(), ws_root))
        .collect();
    let all: Vec<&Subject> = subjects.iter().collect();
    let mut out = Resolved::default();
    assign(
        cfg,
        &all,
        &all,
        &mut Ctx {
            mode,
            out: &mut out,
        },
    )?;
    Ok(out)
}

/// The best brick of `cfg` for `s`, or an overlap error (draft: the first brick wins).
fn best<'c>(cfg: &'c Config, s: &Subject, cx: &mut Ctx) -> Result<Option<&'c str>> {
    let mut best: Option<(Spec, &str)> = None;
    for b in &cfg.bricks {
        for member in &b.members {
            let Some(spec) = matches(member, &b.base, s) else {
                continue;
            };
            match best {
                Some((bs, n)) if bs > spec || (bs == spec && n == b.name) => {}
                Some((bs, n)) if bs == spec => {
                    let msg = format!(
                        "module {} matches bricks '{n}' and '{}' with equal specificity",
                        s.module.id, b.name
                    );
                    cx.fail(CONFIG_OVERLAP, cfg, Some(&s.module.id), msg)?;
                }
                _ => best = Some((spec, b.name.as_str())),
            }
        }
    }
    Ok(best.map(|(_, n)| n))
}

/// Assign `mods` (the modules of this level) to the children of `cfg`, recursively.
fn assign(cfg: &Config, all: &[&Subject], mods: &[&Subject], cx: &mut Ctx) -> Result<()> {
    let root = cfg.path.is_empty();
    if !root {
        outside_parent(cfg, all, mods, cx)?;
    }
    let mut groups: BTreeMap<&str, Vec<&Subject>> = BTreeMap::new();
    for s in mods {
        match best(cfg, s, cx)? {
            Some(n) => groups.entry(n).or_default().push(s),
            None if root => {}
            None => {
                let msg = format!(
                    "module {} is in composite '{}' but in none of its bricks",
                    s.module.id,
                    cfg.path.join("/")
                );
                cx.fail(CONFIG_UNASSIGNED, cfg, Some(&s.module.id), msg)?;
            }
        }
    }
    for (name, group) in groups {
        let Some(b) = cfg.brick(name) else { continue };
        match &b.sub {
            Some(sub) => assign(sub, all, &group, cx)?,
            None => {
                let mut path = cfg.path.clone();
                path.push(name.to_string());
                for s in group {
                    cx.out.paths.insert(s.module.id.clone(), path.clone());
                }
            }
        }
    }
    Ok(())
}

/// A child member that matches a module outside this composite.
fn outside_parent(cfg: &Config, all: &[&Subject], mods: &[&Subject], cx: &mut Ctx) -> Result<()> {
    let inside: std::collections::BTreeSet<&str> =
        mods.iter().map(|s| s.module.id.as_str()).collect();
    for b in &cfg.bricks {
        for member in &b.members {
            if let Some(s) = all.iter().find(|s| {
                !inside.contains(s.module.id.as_str()) && matches(member, &b.base, s).is_some()
            }) {
                let msg = format!(
                    "member '{member}' of brick '{}' matches module {}, which is outside composite '{}'",
                    b.name,
                    s.module.id,
                    cfg.path.join("/")
                );
                cx.fail(CONFIG_MEMBER_OUTSIDE_PARENT, cfg, None, msg)?;
            }
        }
    }
    Ok(())
}
