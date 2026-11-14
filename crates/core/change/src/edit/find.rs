//! Rules and complexity rules named by identity: a rule by `(from, to, kinds, scope)`, a
//! complexity rule by its `bricks` selector. Never by position, so a stale index cannot edit
//! the wrong rule.

use super::rules::{COMPLEXITY, RULES};
use super::table::aot;
use anyhow::{Result, bail};
use brickie_model::codes::{DESIGN_EXISTS, DESIGN_NO_MATCH};
use brickie_model::design::{RuleKey, RuleSpec};
use toml_edit::{DocumentMut, Item, Table};

/// The identity of a rule table: from, to, sorted kinds, scope (`prod` when absent).
#[derive(PartialEq, Eq)]
struct Id {
    from: String,
    to: String,
    kinds: Vec<String>,
    scope: String,
}

fn text(t: &Table, k: &str) -> String {
    t.get(k)
        .and_then(Item::as_str)
        .unwrap_or_default()
        .to_string()
}

fn id(t: &Table) -> Id {
    let mut kinds: Vec<String> = (t.get("kinds").and_then(Item::as_array))
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    kinds.sort();
    let scope = t.get("scope").and_then(Item::as_str).unwrap_or("prod");
    Id {
        from: text(t, "from"),
        to: text(t, "to"),
        kinds,
        scope: scope.to_string(),
    }
}

/// `a -> b [uses] (prod)`.
fn describe(i: &Id) -> String {
    let kinds = match i.kinds.is_empty() {
        true => String::new(),
        false => format!(" [{}]", i.kinds.join(", ")),
    };
    format!("{} -> {}{kinds} ({})", i.from, i.to, i.scope)
}

fn sorted(v: &[String]) -> Vec<String> {
    let mut v = v.to_vec();
    v.sort();
    v
}

fn ids(doc: &DocumentMut) -> Vec<Id> {
    aot(doc, RULES).map_or_else(Vec::new, |a| a.iter().map(id).collect())
}

/// The index of the one rule of `file` that `key` names.
pub fn rule(doc: &DocumentMut, file: &str, key: &RuleKey) -> Result<usize> {
    let all = ids(doc);
    let hits: Vec<usize> = (0..all.len())
        .filter(|&i| {
            let r = &all[i];
            r.from == key.from
                && r.to == key.to
                && key.kinds.as_ref().is_none_or(|k| sorted(k) == r.kinds)
                && key.scope.as_ref().is_none_or(|s| *s == r.scope)
        })
        .collect();
    match hits.as_slice() {
        [i] => Ok(*i),
        [] => {
            let list: Vec<String> = all.iter().map(describe).collect();
            let list = match list.is_empty() {
                true => "none".to_string(),
                false => list.join("; "),
            };
            bail!(
                "{DESIGN_NO_MATCH}: no rule {} -> {} in {file}; its rules: {list}",
                key.from,
                key.to
            )
        }
        many => {
            let list: Vec<String> = many.iter().map(|&i| describe(&all[i])).collect();
            bail!(
                "{DESIGN_NO_MATCH}: {} rules {} -> {} in {file} match: {}; name one with --kind and --scope",
                many.len(),
                key.from,
                key.to,
                list.join("; ")
            )
        }
    }
}

/// Refuse a new rule whose identity is already in `file`.
pub fn new_rule(doc: &DocumentMut, file: &str, r: &RuleSpec) -> Result<()> {
    let new = Id {
        from: r.from.clone(),
        to: r.to.clone(),
        kinds: sorted(&r.kinds),
        scope: r.scope.clone().unwrap_or_else(|| "prod".into()),
    };
    if ids(doc).contains(&new) {
        bail!(
            "{DESIGN_EXISTS}: {file} already has rule {}; change it with `rule set` or remove it first",
            describe(&new)
        );
    }
    Ok(())
}

fn selectors(doc: &DocumentMut) -> Vec<String> {
    aot(doc, COMPLEXITY).map_or_else(Vec::new, |a| {
        a.iter()
            .map(|t| {
                t.get("bricks")
                    .and_then(Item::as_str)
                    .unwrap_or("*")
                    .to_string()
            })
            .collect()
    })
}

/// The index of the one complexity rule of `file` for selector `bricks` (`None`: `*`).
pub fn limit(doc: &DocumentMut, file: &str, bricks: Option<&str>) -> Result<usize> {
    let want = bricks.unwrap_or("*");
    let all = selectors(doc);
    let hits: Vec<usize> = (0..all.len()).filter(|&i| all[i] == want).collect();
    match hits.as_slice() {
        [i] => Ok(*i),
        [] => bail!(
            "{DESIGN_NO_MATCH}: no limit for '{want}' in {file}; its limits: {}",
            if all.is_empty() {
                "none".into()
            } else {
                all.join(", ")
            }
        ),
        many => bail!(
            "{DESIGN_NO_MATCH}: {} limits for '{want}' in {file}; merge them by hand",
            many.len()
        ),
    }
}

/// Refuse a new complexity rule whose selector is already in `file`.
pub fn new_limit(doc: &DocumentMut, file: &str, bricks: Option<&str>) -> Result<()> {
    let want = bricks.unwrap_or("*");
    if selectors(doc).iter().any(|s| s == want) {
        bail!(
            "{DESIGN_EXISTS}: {file} already has a limit for '{want}'; change it with `limit set`"
        );
    }
    Ok(())
}

/// `rule a -> b [uses] (prod)` for the semantic diff.
pub fn rule_label(t: &Table) -> String {
    format!("rule {}", describe(&id(t)))
}
