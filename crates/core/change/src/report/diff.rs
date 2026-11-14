//! The semantic diff between two states of the config tree: one line per element added
//! (`+`), removed (`-`) or changed (`~`, with the fields that changed).

use crate::edit::elements;
use std::collections::{BTreeMap, BTreeSet};
use toml_edit::DocumentMut;

pub type Files = BTreeMap<String, Vec<u8>>;

fn parse(bytes: Option<&Vec<u8>>) -> Vec<(String, String)> {
    let text = bytes.map(|b| String::from_utf8_lossy(b).into_owned());
    text.and_then(|t| t.parse::<DocumentMut>().ok())
        .map(|d| elements(&d))
        .unwrap_or_default()
}

/// `key: old -> new` for each top-level key that differs between two canonical tables.
fn fields(old: &str, new: &str) -> String {
    let a: toml::Table = toml::from_str(old).unwrap_or_default();
    let b: toml::Table = toml::from_str(new).unwrap_or_default();
    let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    let show = |v: Option<&toml::Value>| v.map_or("(none)".to_string(), |v| v.to_string());
    let parts: Vec<String> = keys
        .into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .map(|k| format!("{k}: {} -> {}", show(a.get(k)), show(b.get(k))))
        .collect();
    parts.join(", ")
}

/// Lines from `a` to `b`, file by file in path order.
pub fn diff(a: &Files, b: &Files) -> Vec<String> {
    let mut out = Vec::new();
    let paths: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    for p in paths {
        match (a.get(p), b.get(p)) {
            (None, Some(_)) => out.push(format!("+ {p}: new file")),
            (Some(_), None) => out.push(format!("- {p}: left the design")),
            (Some(x), Some(y)) if x == y => continue,
            _ => {}
        }
        let old = parse(a.get(p));
        let new = parse(b.get(p));
        for (label, c) in &new {
            match old.iter().find(|(l, _)| l == label) {
                None => out.push(format!("+ {p}: {label}")),
                Some((_, o)) if o != c => out.push(format!("~ {p}: {label}: {}", fields(o, c))),
                Some(_) => {}
            }
        }
        for (label, _) in old.iter().filter(|(l, _)| !new.iter().any(|(n, _)| n == l)) {
            out.push(format!("- {p}: {label}"));
        }
        let same_elements = old == new;
        if same_elements && a.contains_key(p) && b.contains_key(p) {
            out.push(format!("~ {p}: comments or layout"));
        }
    }
    out
}
