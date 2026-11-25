//! Peer brick group ops (`[[group]]`).

use super::Docs;
use super::brick::{prune_empty, rename_refs};
use super::rules::{COMPLEXITY, LINKS, RULES, read_exports};
use super::table::{aot, aot_mut, find_named, set_list, set_opt};
use super::via;
use crate::paths::join_rel;
use anyhow::{Context, Result, bail};
use brickie_model::codes::DESIGN_INVALID;
use brickie_model::design::GroupSpec;
use std::collections::BTreeSet;
use toml_edit::{DocumentMut, Item, Table, value};

fn index(doc: &mut DocumentMut, name: &str) -> Result<usize> {
    find_named(aot_mut(doc, &["group"])?, name).with_context(|| format!("no group '{name}'"))
}

fn group_mut<'a>(doc: &'a mut DocumentMut, name: &str) -> Result<&'a mut Table> {
    let i = index(doc, name)?;
    aot_mut(doc, &["group"])?
        .get_mut(i)
        .context("group index out of range")
}

/// Whether `doc` already declares a `[[brick]]` or `[[group]]` named `name`.
pub fn name_taken(doc: &DocumentMut, name: &str) -> bool {
    aot(doc, &["brick"]).is_some_and(|a| find_named(a, name).is_some())
        || aot(doc, &["group"]).is_some_and(|a| find_named(a, name).is_some())
}

/// Append a `[[group]]` table to `doc`.
pub fn add(doc: &mut DocumentMut, spec: &GroupSpec) -> Result<()> {
    if spec.name.trim().is_empty() {
        bail!("a group needs a name");
    }
    if name_taken(doc, &spec.name) {
        bail!("'{}' already exists", spec.name);
    }
    let mut t = Table::new();
    t["name"] = value(spec.name.as_str());
    set_list(&mut t, "tags", &spec.tags);
    set_opt(&mut t, "desc", spec.desc.as_deref());
    aot_mut(doc, &["group"])?.push(t);
    Ok(())
}

/// Find the file declaring `[[group]]` `name`: `file` itself if it declares the group, otherwise
/// any file in `level_files(docs, file)`.
fn declaring_file(docs: &Docs, file: &str, name: &str) -> Result<String> {
    let has_group = |f: &str| {
        docs.get(f)
            .and_then(|d| aot(d, &["group"]))
            .and_then(|a| find_named(a, name))
            .is_some()
    };
    if has_group(file) {
        return Ok(file.to_string());
    }
    level_files(docs, file)
        .into_iter()
        .find(|f| has_group(f))
        .with_context(|| format!("no group '{name}'"))
}

/// Update or remove `tags` and `desc` on `[[group]]` `name` in `file` (or an `include`d file of
/// the same level).
pub fn set(
    docs: &mut Docs,
    file: &str,
    name: &str,
    tags: Option<&[String]>,
    desc: Option<&str>,
) -> Result<()> {
    let target = declaring_file(docs, file, name)?;
    let t = group_mut(docs.get_mut(&target)?, name)?;
    if let Some(v) = tags {
        set_list(t, "tags", v);
    }
    if let Some(d) = desc {
        set_opt(t, "desc", Some(d));
    }
    Ok(())
}

fn include_paths(doc: &DocumentMut, cur: &str) -> Vec<String> {
    let Some(arr) = doc.get("include").and_then(Item::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(|i| join_rel(cur, i)))
        .collect()
}

fn level_roots(docs: &Docs, file: &str) -> BTreeSet<String> {
    let all = docs.files();
    let mut roots = BTreeSet::new();
    let mut queue = vec![file.to_string()];
    let mut seen = BTreeSet::new();
    while let Some(cur) = queue.pop() {
        if !seen.insert(cur.clone()) {
            continue;
        }
        let parents: Vec<String> = all
            .iter()
            .filter(|f| {
                docs.get(f)
                    .is_some_and(|d| include_paths(d, f).iter().any(|inc| inc == &cur))
            })
            .cloned()
            .collect();
        if parents.is_empty() {
            roots.insert(cur);
        } else {
            queue.extend(parents);
        }
    }
    roots
}

/// All files belonging to the same level as `file`: any file that includes `file` (transitively)
/// plus every file reachable via `include` from them.
pub fn level_files(docs: &Docs, file: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut queue: Vec<String> = level_roots(docs, file).into_iter().collect();
    while let Some(cur) = queue.pop() {
        if !seen.insert(cur.clone()) {
            continue;
        }
        let Some(doc) = docs.get(&cur) else {
            continue;
        };
        queue.extend(
            include_paths(doc, &cur)
                .into_iter()
                .filter(|inc| docs.get(inc).is_some()),
        );
    }
    seen.into_iter().collect()
}

fn rename_brick_groups(doc: &mut DocumentMut, name: &str, new_name: &str) {
    let Ok(bricks) = aot_mut(doc, &["brick"]) else {
        return;
    };
    for t in bricks.iter_mut() {
        if t.get("group").and_then(Item::as_str) == Some(name) {
            t["group"] = value(new_name);
        }
    }
}

/// Rename `[[group]]` `name` -> `new_name` in `file`, rewriting `brick.group`, local
/// `rule`/`link`/`exports`/`complexity` refs across the level's included files, and parent
/// `composite/group` links.
pub fn rename(docs: &mut Docs, file: &str, name: &str, new_name: &str) -> Result<()> {
    if new_name.trim().is_empty() {
        bail!("a group needs a name");
    }
    let target = declaring_file(docs, file, name)?;
    let files = level_files(docs, file);
    for f in &files {
        if let Some(doc) = docs.get(f)
            && name_taken(doc, new_name)
        {
            bail!("'{new_name}' already exists in {f}");
        }
    }
    group_mut(docs.get_mut(&target)?, name)?["name"] = value(new_name);
    for f in &files {
        let doc = docs.get_mut(f)?;
        rename_brick_groups(doc, name, new_name);
        rename_refs(doc, name, new_name)?;
    }
    for f in &files {
        via::rename(docs, f, name, new_name)?;
    }
    Ok(())
}

/// Whether `doc` has any direct `rule`, `link`, `complexity` or `exports` reference to `name`.
pub fn has_direct_refs(doc: &DocumentMut, name: &str) -> bool {
    let prefix = format!("{name}/");
    if let Some(rules) = aot(doc, RULES)
        && rules.iter().any(|t| {
            t.get("from").and_then(Item::as_str) == Some(name)
                || t.get("to").and_then(Item::as_str) == Some(name)
        })
    {
        return true;
    }
    if let Some(links) = aot(doc, LINKS)
        && links.iter().any(|t| {
            t.get("from").and_then(Item::as_str) == Some(name)
                || t.get("to")
                    .and_then(Item::as_str)
                    .is_some_and(|v| v == name || v.starts_with(&prefix))
        })
    {
        return true;
    }
    if let Some(limits) = aot(doc, COMPLEXITY)
        && limits
            .iter()
            .any(|t| t.get("bricks").and_then(Item::as_str) == Some(name))
    {
        return true;
    }
    read_exports(doc).iter().any(|e| e == name)
}

/// Remove `[[group]]` `name` from `file`, refusing if any brick or rule/link/export/limit or
/// parent `composite/group` link still references it.
pub fn remove(docs: &mut Docs, file: &str, name: &str) -> Result<()> {
    let target = declaring_file(docs, file, name)?;
    let files = level_files(docs, file);
    for f in &files {
        let Some(doc) = docs.get(f) else {
            continue;
        };
        if let Some(bricks) = aot(doc, &["brick"])
            && let Some(b) = bricks
                .iter()
                .find(|t| t.get("group").and_then(Item::as_str) == Some(name))
        {
            let bname = b.get("name").and_then(Item::as_str).unwrap_or_default();
            bail!(
                "{DESIGN_INVALID}: group '{name}' still contains brick '{bname}' in {f}; \
                 ungroup its bricks first"
            );
        }
        if has_direct_refs(doc, name) {
            bail!(
                "{DESIGN_INVALID}: group '{name}' is still referenced by a rule, link, limit or \
                 export in {f}; remove the references first"
            );
        }
    }
    for f in &files {
        via::guard_child(docs, f, name, "removing it")?;
    }
    let doc = docs.get_mut(&target)?;
    let i = index(doc, name)?;
    aot_mut(doc, &["group"])?.remove(i);
    prune_empty(doc);
    Ok(())
}
