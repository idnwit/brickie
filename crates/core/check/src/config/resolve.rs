//! Resolving every module to a leaf path in the brick tree, and the partition checks.
//!
//! At each level a module may match the members of only one brick; a match in two bricks is
//! `config/overlap`, whatever the members' specificity. Below a composite every module must
//! land in exactly one child (`config/unassigned`), and a child's members must not reach
//! modules outside the composite (`config/member-outside-parent`). At the root an unmatched
//! module is merely unmapped.
//!
//! Every partition error of the tree is collected (best effort goes on: an unassigned module
//! stays unmapped, an overlap goes to the first brick, a member reaching outside is ignored);
//! all of them are returned beside the resolved tree, which is always complete.

use super::Config;
use super::member::{Subject, matches};
use brickie_facts::{Crate, Module};
use brickie_model::codes::{CONFIG_MEMBER_OUTSIDE_PARENT, CONFIG_OVERLAP, CONFIG_UNASSIGNED};
use std::collections::BTreeMap;
use std::path::Path;

/// Module id -> path of brick names from the root to its leaf (`["web", "api"]`). Unmapped
/// modules are absent.
#[derive(Debug, Default, Clone)]
pub struct Resolved {
    /// The paths, keyed by module id.
    pub paths: BTreeMap<String, Vec<String>>,
}

/// One partition error: the code does not fit the drawn structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// `config/unassigned` | `config/overlap` | `config/member-outside-parent`.
    pub code: &'static str,
    /// The full message (`code: ...`).
    pub message: String,
    /// The composite (level path) it happened in; `[]` is the root.
    pub level: Vec<String>,
    /// The config file of that level.
    pub file: String,
    /// The module concerned, when one is.
    pub module: Option<String>,
    /// The member selector concerned, when one is.
    pub member: Option<String>,
}

struct Ctx<'r> {
    out: &'r mut Resolved,
    issues: Vec<Issue>,
}

impl Ctx<'_> {
    /// Record a partition error.
    fn fail(
        &mut self,
        code: &'static str,
        cfg: &Config,
        module: Option<&str>,
        member: Option<&str>,
        msg: String,
    ) {
        let message = format!("{code}: {msg} (level {}, {})", cfg.level(), cfg.file);
        self.issues.push(Issue {
            code,
            message,
            level: cfg.path.clone(),
            file: cfg.file.clone(),
            module: module.map(str::to_string),
            member: member.map(str::to_string),
        });
    }
}

impl Resolved {
    /// The leaf path of a module.
    pub fn path(&self, module: &str) -> Option<&[String]> {
        self.paths.get(module).map(Vec::as_slice)
    }
}

/// Resolve every module of `modules` to its leaf path in the tree `cfg`. `ws_root` is the
/// workspace root that `path:` members are relative to. Returns the resolved tree (always
/// complete) and every partition error found; see the module docs for the rules.
pub fn resolve(
    cfg: &Config,
    crates: &[Crate],
    modules: &[Module],
    ws_root: &Path,
) -> (Resolved, Vec<Issue>) {
    let by_id: BTreeMap<&str, &Crate> = crates.iter().map(|c| (c.id.as_str(), c)).collect();
    let subjects: Vec<Subject> = modules
        .iter()
        .map(|m| Subject::new(m, by_id.get(m.krate.as_str()).copied(), ws_root))
        .collect();
    let all: Vec<&Subject> = subjects.iter().collect();
    let mut out = Resolved::default();
    let mut cx = Ctx {
        out: &mut out,
        issues: Vec::new(),
    };
    assign(cfg, &all, &all, &mut cx);
    let issues = cx.issues;
    (out, issues)
}

/// The brick of `cfg` whose members match `s`; a match in a second brick is an overlap, which is
/// recorded and the first brick keeps the module.
fn owner<'c>(cfg: &'c Config, s: &Subject, cx: &mut Ctx) -> Option<&'c str> {
    let mut found: Option<&str> = None;
    for b in &cfg.bricks {
        let Some(member) = b.members.iter().find(|m| matches(m, &b.base, s)) else {
            continue;
        };
        match found {
            None => found = Some(b.name.as_str()),
            Some(n) => {
                let msg = format!(
                    "module {} matches bricks '{n}' and '{}'; a module belongs to one brick",
                    s.module.id, b.name
                );
                cx.fail(CONFIG_OVERLAP, cfg, Some(&s.module.id), Some(member), msg);
            }
        }
    }
    found
}

/// Assign `mods` (the modules of this level) to the children of `cfg`, recursively.
fn assign(cfg: &Config, all: &[&Subject], mods: &[&Subject], cx: &mut Ctx) {
    let root = cfg.path.is_empty();
    if !root {
        outside_parent(cfg, all, mods, cx);
    }
    let mut groups: BTreeMap<&str, Vec<&Subject>> = BTreeMap::new();
    for s in mods {
        match owner(cfg, s, cx) {
            Some(n) => {
                groups.entry(n).or_default().push(s);
            }
            None if root => {}
            None => {
                let msg = format!(
                    "module {} is in composite '{}' but in none of its bricks",
                    s.module.id,
                    cfg.path.join("/")
                );
                cx.fail(CONFIG_UNASSIGNED, cfg, Some(&s.module.id), None, msg);
            }
        }
    }
    for (name, group) in groups {
        let Some(b) = cfg.brick(name) else { continue };
        match &b.sub {
            Some(sub) => assign(sub, all, &group, cx),
            None => {
                let mut path = cfg.path.clone();
                path.push(name.to_string());
                for s in group {
                    cx.out.paths.insert(s.module.id.clone(), path.clone());
                }
            }
        }
    }
}

/// A child member that matches a module outside this composite.
fn outside_parent(cfg: &Config, all: &[&Subject], mods: &[&Subject], cx: &mut Ctx) {
    let inside: std::collections::BTreeSet<&str> =
        mods.iter().map(|s| s.module.id.as_str()).collect();
    for b in &cfg.bricks {
        for member in &b.members {
            if let Some(s) = all
                .iter()
                .find(|s| !inside.contains(s.module.id.as_str()) && matches(member, &b.base, s))
            {
                let msg = format!(
                    "member '{member}' of brick '{}' matches module {}, which is outside composite '{}' (a composite owns the modules under its path)",
                    b.name,
                    s.module.id,
                    cfg.path.join("/")
                );
                cx.fail(
                    CONFIG_MEMBER_OUTSIDE_PARENT,
                    cfg,
                    Some(&s.module.id),
                    Some(member),
                    msg,
                );
            }
        }
    }
}
