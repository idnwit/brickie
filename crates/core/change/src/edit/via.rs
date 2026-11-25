//! Child-link bookkeeping: a link in a parent file may name an exported child of the composite
//! it targets (`to = "ports/in"`). Ops on the child level must keep those names valid.

use super::Docs;
use super::rules::LINKS;
use super::table::{aot, aot_mut};
use crate::paths::join_rel;
use anyhow::{Result, bail};
use brickie_model::codes::DESIGN_INVALID;
use toml_edit::{Item, value};

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

/// Whether link `t` targets `composite/child`.
fn names(t: &toml_edit::Table, composite: &str, child: &str) -> bool {
    t.get("to").and_then(Item::as_str) == Some(&format!("{composite}/{child}"))
}

/// Rewrite links `composite/from` -> `composite/to` that target the composite owning
/// `child_file` (a child brick or group was renamed).
pub fn rename(docs: &mut Docs, child_file: &str, from: &str, to: &str) -> Result<()> {
    for (file, composite) in owners(docs, child_file) {
        for f in super::group::level_files(docs, &file) {
            if docs
                .get(&f)
                .and_then(|d| aot(d, LINKS))
                .is_none_or(|l| !l.iter().any(|t| names(t, &composite, from)))
            {
                continue;
            }
            let links = aot_mut(docs.get_mut(&f)?, LINKS)?;
            for t in links.iter_mut().filter(|t| names(t, &composite, from)) {
                t["to"] = value(format!("{composite}/{to}"));
            }
            super::brick::prune_empty(docs.get_mut(&f)?);
        }
    }
    Ok(())
}

/// Fail when a link names `composite/child` of the composite owning `child_file`, and `child`
/// is about to leave that level's exports (`why` says how).
pub fn guard_child(docs: &Docs, child_file: &str, child: &str, why: &str) -> Result<()> {
    for (file, composite) in owners(docs, child_file) {
        for f in super::group::level_files(docs, &file) {
            let Some(links) = docs.get(&f).and_then(|d| aot(d, LINKS)) else {
                continue;
            };
            if links.iter().any(|t| names(t, &composite, child)) {
                bail!(
                    "{DESIGN_INVALID}: a link in {f} goes to '{composite}/{child}'; {why} would \
                     leave it dangling, edit or remove the link first"
                );
            }
        }
    }
    Ok(())
}

/// Fail when a link of `file` (or an `include`d file of the same level) names a child of composite
/// `name` (it is about to become a leaf).
pub fn guard_composite(docs: &Docs, file: &str, name: &str, why: &str) -> Result<()> {
    let prefix = format!("{name}/");
    for f in super::group::level_files(docs, file) {
        let Some(links) = docs.get(&f).and_then(|d| aot(d, LINKS)) else {
            continue;
        };
        for t in links.iter() {
            let to = t.get("to").and_then(Item::as_str).unwrap_or_default();
            if to.starts_with(&prefix) {
                bail!(
                    "{DESIGN_INVALID}: a link in {f} goes to '{to}'; {why} would leave it \
                     dangling, edit or remove the link first"
                );
            }
        }
    }
    Ok(())
}
