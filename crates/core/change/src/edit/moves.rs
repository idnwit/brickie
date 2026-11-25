//! Bricks drawn by kind, and `move_brick` (reparenting into another level).

use super::Docs;
use super::brick::{self, brick_mut, insert, rename_refs, take, unexport};
use super::group::has_direct_refs;
use super::table::{aot, find_named};
use crate::paths::{dir_of, join_dir, join_rel, relative};
use anyhow::{Context, Result, bail};
use brickie_model::codes::DESIGN_INVALID;
use brickie_model::design::{BrickKind, BrickSpec, Op};
use toml_edit::{Item, Table, value};

/// `add_brick`: `kind` fills in `members` (crate) or `path` (composite, whose file is created).
pub fn add_brick(docs: &mut Docs, file: &str, spec: &BrickSpec) -> Result<()> {
    let mut spec = spec.clone();
    let unset = spec.members.is_empty() && spec.path.is_none();
    match spec.kind {
        Some(BrickKind::Crate) if unset => {
            let dir = if dir_of(file).is_empty() {
                "crates/"
            } else {
                ""
            };
            spec.members = vec![format!("path:{dir}{}/", spec.name)];
        }
        Some(BrickKind::Composite) if spec.path.is_none() => spec.path = Some(spec.name.clone()),
        _ => {}
    }
    brick::add(docs.get_mut(file)?, &spec)?;
    if let (Some(BrickKind::Composite), Some(p)) = (spec.kind, &spec.path) {
        let new = join_rel(file, &format!("{}/brickie.toml", p.trim_end_matches('/')));
        docs.create_level(&new)?;
    }
    Ok(())
}

/// The level file a `move_brick` moves into (once applied, for `into` bricks).
pub fn target(docs: &Docs, op: &Op) -> Option<String> {
    let Op::MoveBrick {
        into_file, into, ..
    } = op
    else {
        return None;
    };
    let Some(parent) = into else {
        return Some(into_file.clone());
    };
    let bricks = docs.get(into_file)?.get("brick")?.as_array_of_tables()?;
    let t = bricks
        .iter()
        .find(|t| t.get("name").and_then(Item::as_str) == Some(parent))?;
    let p = t.get("path")?.as_str()?.trim_end_matches('/');
    Some(join_rel(into_file, &format!("{p}/brickie.toml")))
}

pub fn move_brick(docs: &mut Docs, op: &Op) -> Result<()> {
    let Op::MoveBrick {
        file,
        name,
        into_file,
        into,
        path,
    } = op
    else {
        bail!("not a move_brick");
    };
    if into.as_deref() == Some(name.as_str()) && into_file == file {
        bail!("cannot move brick '{name}' into itself");
    }
    super::via::guard_child(docs, file, name, "moving it")?;
    if let Some(g) = docs
        .get(file)
        .and_then(|d| aot(d, &["brick"]))
        .and_then(|a| find_named(a, name).and_then(|i| a.get(i)))
        .and_then(|t| t.get("group").and_then(Item::as_str))
        .filter(|g| !g.is_empty())
    {
        bail!("{DESIGN_INVALID}: brick '{name}' is in group '{g}'; ungroup it first before moving");
    }
    if let Some(parent) = into {
        if docs
            .get(into_file)
            .and_then(|d| aot(d, &["group"]))
            .is_some_and(|a| find_named(a, parent).is_some())
        {
            bail!(
                "{DESIGN_INVALID}: '{parent}' is a group, not a composite; \
                 use `brick set --group {parent}` instead"
            );
        }
        let parent_grouped = docs
            .get(into_file)
            .and_then(|d| aot(d, &["brick"]))
            .and_then(|a| find_named(a, parent).and_then(|i| a.get(i)))
            .and_then(|t| t.get("group").and_then(Item::as_str))
            .is_some_and(|g| !g.is_empty());
        if parent_grouped
            && into_file == file
            && docs.get(file).is_some_and(|d| has_direct_refs(d, name))
        {
            bail!(
                "{DESIGN_INVALID}: cannot move '{name}' into grouped composite '{parent}' while \
                 '{name}' has direct rules, links, limits or exports"
            );
        }
        let t = brick_mut(docs.get_mut(into_file)?, parent)?;
        if !t.contains_key("path") {
            let p = path.clone().unwrap_or_else(|| parent.clone());
            t["path"] = value(p.trim_end_matches('/'));
            let new = join_rel(
                into_file,
                &format!("{}/brickie.toml", p.trim_end_matches('/')),
            );
            docs.create_level(&new)?;
        }
    }
    let Some(to) = target(docs, op) else {
        bail!("no level to move '{name}' into");
    };
    if &to == file {
        bail!("brick '{name}' is already in {to}");
    }
    let src = docs.get_mut(file)?;
    let mut t = take(src, name)?;
    match into {
        Some(parent) if into_file == file => {
            rename_refs(src, name, parent)?;
            drop_self_rules(src, parent)?;
        }
        _ => unexport(src, name),
    }
    rebase(&mut t, &dir_of(file), &dir_of(&to));
    insert(docs.get_mut(&to)?, t)
}

/// Rules and links between two children of one composite become `parent -> parent` once
/// renamed: drop them.
fn drop_self_rules(doc: &mut toml_edit::DocumentMut, parent: &str) -> Result<()> {
    drop_self(doc, &["rule"], parent)?;
    drop_self(doc, &["link"], parent)
}

fn drop_self(doc: &mut toml_edit::DocumentMut, table: &[&str], parent: &str) -> Result<()> {
    let rules = super::table::aot_mut(doc, table)?;
    let is_str = |t: &Table, k: &str| t.get(k).and_then(Item::as_str) == Some(parent);
    let mut i = 0;
    while i < rules.len() {
        let t = rules.get(i).context("rule index")?;
        if is_str(t, "from") && is_str(t, "to") {
            rules.remove(i);
        } else {
            i += 1;
        }
    }
    brick::prune_empty(doc);
    Ok(())
}

/// Keep `path` and `path:` members pointing at the same folders from the new directory.
pub(super) fn rebase(t: &mut Table, from: &str, to: &str) {
    let move_dir = |p: &str| relative(to, &join_dir(from, p));
    if let Some(p) = t.get("path").and_then(Item::as_str).map(move_dir) {
        t["path"] = value(p);
    }
    let Some(members) = t.get_mut("members").and_then(Item::as_array_mut) else {
        return;
    };
    for v in members.iter_mut() {
        if let Some(p) = v.as_str().and_then(|s| s.strip_prefix("path:")) {
            *v = format!("path:{}/", move_dir(p)).into();
        }
    }
}
