//! Brick ops on one document.

use super::table::{aot_mut, find_named, set_list, set_opt};
use anyhow::{Context, Result, bail};
use brickie_model::design::BrickSpec;
use toml_edit::{DocumentMut, Item, Table, Value, value};

fn index(doc: &mut DocumentMut, name: &str) -> Result<usize> {
    find_named(aot_mut(doc, &["brick"])?, name).with_context(|| format!("no brick '{name}'"))
}

pub fn brick_mut<'a>(doc: &'a mut DocumentMut, name: &str) -> Result<&'a mut Table> {
    let i = index(doc, name)?;
    aot_mut(doc, &["brick"])?
        .get_mut(i)
        .context("brick index out of range")
}

/// Remove a brick and return its table.
pub fn take(doc: &mut DocumentMut, name: &str) -> Result<Table> {
    let t = brick_mut(doc, name)?.clone();
    remove(doc, name)?;
    Ok(t)
}

/// Append a brick table (its name must be new in `doc`).
pub fn insert(doc: &mut DocumentMut, t: Table) -> Result<()> {
    let name = t
        .get("name")
        .and_then(Item::as_str)
        .unwrap_or_default()
        .to_string();
    let bricks = aot_mut(doc, &["brick"])?;
    if find_named(bricks, &name).is_some() {
        bail!("brick '{name}' already exists there");
    }
    bricks.push(t);
    Ok(())
}

/// The brick names in `exports`.
pub fn exported(doc: &DocumentMut) -> Vec<String> {
    let Some(a) = doc.get("exports").and_then(Item::as_array) else {
        return Vec::new();
    };
    a.iter()
        .filter_map(|v| {
            v.as_str()
                .or_else(|| v.as_inline_table()?.get("brick")?.as_str())
        })
        .map(String::from)
        .collect()
}

/// Drop `name` from `exports`.
pub fn unexport(doc: &mut DocumentMut, name: &str) {
    if let Some(a) = doc.get_mut("exports").and_then(Item::as_array_mut) {
        a.retain(|v| {
            let n = v
                .as_str()
                .or_else(|| v.as_inline_table()?.get("brick")?.as_str());
            n != Some(name)
        });
    }
}

pub fn add(doc: &mut DocumentMut, spec: &BrickSpec) -> Result<()> {
    if spec.name.trim().is_empty() {
        bail!("a brick needs a name");
    }
    let bricks = aot_mut(doc, &["brick"])?;
    if find_named(bricks, &spec.name).is_some() {
        bail!("brick '{}' already exists", spec.name);
    }
    let mut t = Table::new();
    t["name"] = value(spec.name.as_str());
    set_list(&mut t, "members", &spec.members);
    set_list(&mut t, "tags", &spec.tags);
    set_opt(&mut t, "path", spec.path.as_deref());
    set_opt(&mut t, "desc", spec.desc.as_deref());
    bricks.push(t);
    Ok(())
}

pub fn remove(doc: &mut DocumentMut, name: &str) -> Result<()> {
    let i = index(doc, name)?;
    aot_mut(doc, &["brick"])?.remove(i);
    Ok(())
}

/// The fields of `set_brick`; `None` leaves a field as it is.
pub struct Fields<'a> {
    pub desc: Option<&'a str>,
    pub tags: Option<&'a [String]>,
    pub members: Option<&'a [String]>,
    pub path: Option<&'a str>,
}

pub fn set(doc: &mut DocumentMut, name: &str, f: Fields) -> Result<()> {
    let t = brick_mut(doc, name)?;
    if let Some(d) = f.desc {
        set_opt(t, "desc", Some(d));
    }
    if let Some(p) = f.path {
        set_opt(t, "path", Some(p));
    }
    if let Some(v) = f.tags {
        set_list(t, "tags", v);
    }
    if let Some(v) = f.members {
        set_list(t, "members", v);
    }
    Ok(())
}

/// Rename a brick and every reference to it in the same file: rule selectors, complexity
/// selectors and `exports` entries.
pub fn rename(doc: &mut DocumentMut, from: &str, to: &str) -> Result<()> {
    if find_named(aot_mut(doc, &["brick"])?, to).is_some() {
        bail!("brick '{to}' already exists");
    }
    brick_mut(doc, from)?["name"] = value(to);
    rename_refs(doc, from, to)
}

/// Point the rule selectors, complexity selectors and `exports` of `doc` at `to`.
pub fn rename_refs(doc: &mut DocumentMut, from: &str, to: &str) -> Result<()> {
    for t in aot_mut(doc, &["deps", "rule"])?.iter_mut() {
        rename_key(t, "from", from, to);
        rename_key(t, "to", from, to);
    }
    for t in aot_mut(doc, &["complexity"])?.iter_mut() {
        rename_key(t, "bricks", from, to);
    }
    prune_empty(doc);
    if let Some(a) = doc.get_mut("exports").and_then(Item::as_array_mut) {
        for v in a.iter_mut() {
            rename_export(v, from, to);
        }
    }
    Ok(())
}

fn rename_key(t: &mut Table, key: &str, from: &str, to: &str) {
    if t.get(key).and_then(Item::as_str) == Some(from) {
        t[key] = value(to);
    }
}

fn rename_export(v: &mut Value, from: &str, to: &str) {
    match v {
        Value::String(s) if s.value() == from => *v = Value::from(to),
        Value::InlineTable(t) if t.get("brick").and_then(Value::as_str) == Some(from) => {
            t.insert("brick", Value::from(to));
        }
        _ => {}
    }
}

/// Drop the empty `[[deps.rule]]` / `[[complexity]]` arrays and implicit `[deps]` that
/// `aot_mut` created while looking.
pub fn prune_empty(doc: &mut DocumentMut) {
    let empty = |i: &Item| i.as_array_of_tables().is_some_and(|a| a.is_empty());
    if doc.get("complexity").is_some_and(empty) {
        doc.remove("complexity");
    }
    if doc.get("brick").is_some_and(empty) {
        doc.remove("brick");
    }
    if let Some(deps) = doc.get_mut("deps").and_then(Item::as_table_mut) {
        if deps.get("rule").is_some_and(empty) {
            deps.remove("rule");
        }
        if deps.is_empty() && deps.is_implicit() {
            doc.remove("deps");
        }
    }
}
