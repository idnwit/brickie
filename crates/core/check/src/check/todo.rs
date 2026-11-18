//! Planned work: what the design has and the code lacks. Never a finding.

use crate::config::{Config, LinkState, Resolved, level_name};
use crate::rules;
use brickie_facts::Cache;
use std::collections::BTreeSet;

/// What the design has and the code lacks: informational, never a finding and never counted in
/// an exit code.
#[derive(Debug, Default, Clone)]
pub struct Todo {
    /// Paths (from the root) of the bricks with no measured module, in tree order.
    pub bricks: Vec<Vec<String>>,
    /// Valid links no measured edge goes through yet.
    pub links: Vec<LinkState>,
    /// Links on a cycle of their level's links.
    pub cycles: Vec<LinkState>,
}

impl Todo {
    /// One line per item.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for p in &self.bricks {
            out.push(format!("planned brick {} has no code yet", level_name(p)));
        }
        for s in &self.links {
            out.push(format!(
                "planned {} has no code edge yet ({})",
                rules::named(s),
                s.match_text()
            ));
        }
        for s in &self.cycles {
            out.push(format!("{} is on a cycle of links", rules::named(s)));
        }
        out
    }
}

pub(super) fn todo(cache: &Cache, cfg: &Config, res: &Resolved) -> Todo {
    Todo {
        bricks: planned(cfg, res),
        links: rules::planned_links(cache, cfg, res),
        cycles: (rules::design_links(cfg).into_iter())
            .filter(|s| s.cycle)
            .collect(),
    }
}

/// Paths (from the root) of the bricks with no measured module, in tree order.
fn planned(cfg: &Config, resolved: &Resolved) -> Vec<Vec<String>> {
    let mut built: BTreeSet<&[String]> = BTreeSet::new();
    for path in resolved.paths.values() {
        for n in 1..=path.len() {
            built.insert(&path[..n]);
        }
    }
    let mut out = Vec::new();
    walk(cfg, &built, &mut out);
    out
}

fn walk(level: &Config, built: &BTreeSet<&[String]>, out: &mut Vec<Vec<String>>) {
    for b in &level.bricks {
        let mut path = level.path.clone();
        path.push(b.name.clone());
        if !built.contains(path.as_slice()) {
            out.push(path);
        }
        if let Some(sub) = &b.sub {
            walk(sub, built, out);
        }
    }
}
