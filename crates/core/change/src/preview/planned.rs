//! Planned bricks: bricks of the design no measured module maps to yet.

use brickie_check::config::{Config, Resolved};
use std::collections::BTreeSet;

/// Paths (from the root) of the bricks with no measured module, in tree order.
pub fn planned(cfg: &Config, resolved: &Resolved) -> Vec<Vec<String>> {
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
