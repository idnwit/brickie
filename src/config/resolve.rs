//! Brick mapping: which module belongs to which brick, and the hash used for staleness.

use super::Brick;
use crate::cache::{Crate, Module, sha256_hex};
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

/// Hash of the brick mapping; a change makes a cache stale.
pub fn hash(bricks: &[Brick]) -> String {
    sha256_hex(serde_json::to_string(bricks).unwrap_or_default().as_bytes())
}

/// Resolve every module to at most one brick. The most specific member wins (a module id beats
/// a package name, a longer module id beats a shorter one); equal specificity across different
/// bricks is an error. Returns the mapping and members that matched no module.
pub fn resolve(
    bricks: &[Brick],
    crates: &[Crate],
    modules: &[Module],
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let package_of: BTreeMap<&str, &str> = crates
        .iter()
        .map(|c| (c.id.as_str(), c.package.as_str()))
        .collect();
    let mut map = BTreeMap::new();
    let mut used = BTreeSet::new();
    for m in modules {
        let package = package_of.get(m.krate.as_str()).copied().unwrap_or("");
        let mut best: Option<(usize, &str)> = None;
        for b in bricks {
            for member in &b.members {
                let spec = if member == package {
                    1
                } else if m.id == *member
                    || m.id
                        .strip_prefix(member.as_str())
                        .is_some_and(|r| r.starts_with("::"))
                {
                    2 + member.matches("::").count()
                } else {
                    continue;
                };
                used.insert((b.name.as_str(), member.as_str()));
                match best {
                    Some((s, n)) if s > spec || (s == spec && n == b.name) => {}
                    Some((s, n)) if s == spec => bail!(
                        "module {} matches bricks '{n}' and '{}' with equal specificity",
                        m.id,
                        b.name
                    ),
                    _ => best = Some((spec, b.name.as_str())),
                }
            }
        }
        if let Some((_, n)) = best {
            map.insert(m.id.clone(), n.to_string());
        }
    }
    let unmatched = bricks
        .iter()
        .flat_map(|b| b.members.iter().map(move |m| (b.name.as_str(), m.as_str())))
        .filter(|k| !used.contains(k))
        .map(|(b, m)| format!("{b}: {m}"))
        .collect();
    Ok((map, unmatched))
}
