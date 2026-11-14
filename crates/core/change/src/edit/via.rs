//! `via` bookkeeping: a rule in a parent file may name exported children of the composite
//! it targets (`via = ["ports-in"]`). Ops on the child level must keep those names valid.

use super::Docs;
use super::table::{aot, aot_mut};
use crate::paths::join_rel;
use anyhow::{Result, bail};
use brickie_model::codes::DESIGN_INVALID;
use toml_edit::{Item, Value};

/// The `(file, composite)` pairs whose `path` level is `child_file`.
fn owners(docs: &Docs, child_file: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for f in docs.files() {
        let Some(bricks) = docs.get(&f).and_then(|d| aot(d, &["brick"])) else {
            continue;
        };
        for t in bricks.iter() {
            let Some(p) = t.get("path").and_then(Item::as_str) else {
                continue;
            };
            let level = join_rel(&f, &format!("{}/brickie.toml", p.trim_end_matches('/')));
            if level == child_file {
                let name = t.get("name").and_then(Item::as_str).unwrap_or_default();
                out.push((f.clone(), name.to_string()));
            }
        }
    }
    out
}

fn names(t: &toml_edit::Table, via: &str) -> bool {
    t.get("via")
        .and_then(Item::as_array)
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(via)))
}

/// Rewrite `via` entries `from` -> `to` in the rules that target the composite owning
/// `child_file` (a child brick was renamed).
pub fn rename(docs: &mut Docs, child_file: &str, from: &str, to: &str) -> Result<()> {
    for (file, composite) in owners(docs, child_file) {
        let rules = aot_mut(docs.get_mut(&file)?, &["deps", "rule"])?;
        for t in rules.iter_mut() {
            if t.get("to").and_then(Item::as_str) != Some(composite.as_str()) {
                continue;
            }
            if let Some(a) = t.get_mut("via").and_then(Item::as_array_mut) {
                for v in a.iter_mut().filter(|v| v.as_str() == Some(from)) {
                    *v = Value::from(to);
                }
            }
        }
        super::brick::prune_empty(docs.get_mut(&file)?);
    }
    Ok(())
}

/// Fail when a rule's `via` names `child` of the composite owning `child_file`, and `child`
/// is about to leave that level's exports (`why` says how).
pub fn guard_child(docs: &Docs, child_file: &str, child: &str, why: &str) -> Result<()> {
    for (file, composite) in owners(docs, child_file) {
        let Some(rules) = docs.get(&file).and_then(|d| aot(d, &["deps", "rule"])) else {
            continue;
        };
        for (i, t) in rules.iter().enumerate() {
            let to = t.get("to").and_then(Item::as_str);
            if to == Some(composite.as_str()) && names(t, child) {
                bail!(
                    "{DESIGN_INVALID}: {file} rule {i} (to '{composite}') names '{child}' in `via`; \
                     {why} would leave it dangling, edit or remove the rule first"
                );
            }
        }
    }
    Ok(())
}

/// Fail when a rule of `file` with `via` targets composite `name` (it is about to become a
/// leaf).
pub fn guard_composite(docs: &Docs, file: &str, name: &str, why: &str) -> Result<()> {
    let Some(rules) = docs.get(file).and_then(|d| aot(d, &["deps", "rule"])) else {
        return Ok(());
    };
    for (i, t) in rules.iter().enumerate() {
        if t.get("to").and_then(Item::as_str) == Some(name) && t.contains_key("via") {
            bail!(
                "{DESIGN_INVALID}: {file} rule {i} (to '{name}') has `via`; {why} would leave it \
                 dangling, edit or remove the rule first"
            );
        }
    }
    Ok(())
}
