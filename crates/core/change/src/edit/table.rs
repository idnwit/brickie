//! Small `toml_edit` helpers shared by the op implementations.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, Value, value};

/// The array of tables at `path` (`["brick"]`, `["deps", "rule"]`), created when absent.
pub fn aot_mut<'a>(doc: &'a mut DocumentMut, path: &[&str]) -> Result<&'a mut ArrayOfTables> {
    let (last, parents) = path.split_last().context("empty path")?;
    let mut t: &mut Table = doc.as_table_mut();
    for p in parents {
        let entry = t.entry(p).or_insert_with(|| {
            let mut n = Table::new();
            n.set_implicit(true);
            Item::Table(n)
        });
        t = entry
            .as_table_mut()
            .with_context(|| format!("`{p}` is not a table"))?;
    }
    t.entry(last)
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .with_context(|| format!("`{last}` is not an array of tables"))
}

/// The array of tables at `path`, if present.
pub fn aot<'a>(doc: &'a DocumentMut, path: &[&str]) -> Option<&'a ArrayOfTables> {
    let mut item: &Item = doc.as_item();
    for p in path {
        item = item.get(p)?;
    }
    item.as_array_of_tables()
}

pub fn str_array(items: &[String]) -> Value {
    let mut a = Array::new();
    for s in items {
        a.push(s.as_str());
    }
    Value::Array(a)
}

/// Set `key` to a string, or remove it for `None` / `""`.
pub fn set_opt(t: &mut Table, key: &str, v: Option<&str>) {
    match v.filter(|s| !s.is_empty()) {
        Some(s) => t[key] = value(s),
        None => {
            t.remove(key);
        }
    }
}

/// Set `key` to a string array, or remove it when empty.
pub fn set_list(t: &mut Table, key: &str, v: &[String]) {
    if v.is_empty() {
        t.remove(key);
    } else {
        t[key] = Item::Value(str_array(v));
    }
}

/// Canonical TOML of some key/value items: keys sorted, comments and layout dropped.
pub fn canon<'a>(items: impl Iterator<Item = (&'a str, &'a Item)>) -> String {
    let mut d = DocumentMut::new();
    for (k, it) in items {
        d[k] = it.clone();
    }
    let map: BTreeMap<String, toml::Value> = toml::from_str(&d.to_string()).unwrap_or_default();
    toml::to_string(&map).unwrap_or_default()
}

/// Index of the table in `aot` whose `name` is `name`.
pub fn find_named(aot: &ArrayOfTables, name: &str) -> Option<usize> {
    aot.iter()
        .position(|t| t.get("name").and_then(Item::as_str) == Some(name))
}
