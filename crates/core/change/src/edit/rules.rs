//! Rule, complexity, deps, exports and include ops on one document.

use super::table::{aot_mut, set_list, set_opt, str_array};
use anyhow::{Context, Result, bail};
use brickie_model::design::{ComplexitySpec, ExportSpec, RulePatch, RuleSpec};

pub const RULES: &[&str] = &["deps", "rule"];
pub const COMPLEXITY: &[&str] = &["complexity"];
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value, value};

pub fn rule_table(r: &RuleSpec) -> Table {
    let mut t = Table::new();
    t["from"] = value(r.from.as_str());
    t["to"] = value(r.to.as_str());
    set_list(&mut t, "via", &r.via);
    set_list(&mut t, "kinds", &r.kinds);
    set_opt(&mut t, "scope", r.scope.as_deref());
    t["action"] = value(r.action.as_str());
    set_opt(&mut t, "reason", r.reason.as_deref());
    set_opt(&mut t, "help", r.help.as_deref());
    t
}

pub fn complexity_table(r: &ComplexitySpec) -> Table {
    let mut t = Table::new();
    set_opt(&mut t, "bricks", r.bricks.as_deref());
    let metrics = [
        ("cognitive", r.cognitive),
        ("cyclomatic", r.cyclomatic),
        ("nesting", r.nesting),
        ("paths", r.paths),
        ("params", r.params),
        ("lines", r.lines),
    ];
    for (k, v) in metrics {
        if let Some(v) = v {
            t[k] = value(v as i64);
        }
    }
    set_opt(&mut t, "level", r.level.as_deref());
    set_opt(&mut t, "scope", r.scope.as_deref());
    set_opt(&mut t, "help", r.help.as_deref());
    t
}

/// Push (`index` `None`), replace or (`table` `None`) remove an entry of an array of tables.
pub fn edit_list(
    doc: &mut DocumentMut,
    path: &[&str],
    index: Option<usize>,
    table: Option<Table>,
) -> Result<()> {
    let list = aot_mut(doc, path)?;
    let Some(i) = index else {
        list.push(table.context("nothing to add")?);
        return Ok(());
    };
    let what = path.join(".");
    let slot = list.get_mut(i).with_context(|| format!("no {what}[{i}]"))?;
    match table {
        Some(mut t) => {
            t.decor_mut().clone_from(slot.decor());
            *slot = t;
        }
        None => {
            list.remove(i);
        }
    }
    super::brick::prune_empty(doc);
    Ok(())
}

/// Change the fields of rule `i` that the patch names, keeping the others and their comments.
pub fn patch_rule(doc: &mut DocumentMut, i: usize, p: &RulePatch) -> Result<()> {
    if let Some(a) = p.action.as_deref()
        && a != "allow"
        && a != "deny"
    {
        bail!("action must be allow or deny, not '{a}'");
    }
    let t = aot_mut(doc, RULES)?.get_mut(i).context("no such rule")?;
    if let Some(a) = &p.action {
        t["action"] = value(a.as_str());
    }
    if let Some(v) = &p.via {
        set_list(t, "via", v);
    }
    if let Some(r) = &p.reason {
        set_opt(t, "reason", Some(r));
    }
    if let Some(h) = &p.help {
        set_opt(t, "help", Some(h));
    }
    Ok(())
}

/// Change the fields of complexity rule `i` that the patch names (`0` / `""` removes one).
pub fn patch_complexity(doc: &mut DocumentMut, i: usize, p: &ComplexitySpec) -> Result<()> {
    let t = aot_mut(doc, COMPLEXITY)?
        .get_mut(i)
        .context("no such complexity rule")?;
    let metrics = [
        ("cognitive", p.cognitive),
        ("cyclomatic", p.cyclomatic),
        ("nesting", p.nesting),
        ("paths", p.paths),
        ("params", p.params),
        ("lines", p.lines),
    ];
    for (k, v) in metrics {
        match v {
            Some(0) => {
                t.remove(k);
            }
            Some(v) => t[k] = value(v as i64),
            None => {}
        }
    }
    for (k, v) in [("level", &p.level), ("scope", &p.scope), ("help", &p.help)] {
        if let Some(v) = v {
            set_opt(t, k, Some(v));
        }
    }
    Ok(())
}

pub fn set_deps(doc: &mut DocumentMut, default: Option<&str>, cycles: Option<&str>) -> Result<()> {
    for v in [default, cycles].into_iter().flatten() {
        if v != "allow" && v != "deny" {
            bail!("deps value must be allow or deny, not '{v}'");
        }
    }
    let deps = doc
        .entry("deps")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("`deps` is not a table")?;
    deps.set_implicit(false);
    if let Some(d) = default {
        deps["default"] = value(d);
    }
    if let Some(c) = cycles {
        deps["cycles"] = value(c);
    }
    Ok(())
}

pub fn set_exports(doc: &mut DocumentMut, exports: &[ExportSpec]) {
    if exports.is_empty() {
        doc.remove("exports");
        return;
    }
    let mut a = Array::new();
    for e in exports {
        match &e.scope {
            None => a.push(e.brick.as_str()),
            Some(s) => {
                let mut t = InlineTable::new();
                t.insert("brick", Value::from(e.brick.as_str()));
                t.insert("scope", Value::from(s.as_str()));
                a.push(t);
            }
        }
    }
    doc["exports"] = Item::Value(Value::Array(a));
}

/// Add or remove an `include` entry.
pub fn edit_include(doc: &mut DocumentMut, path: &str, add: bool) -> Result<()> {
    let mut list: Vec<String> = doc
        .get("include")
        .and_then(Item::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let present = list.iter().any(|p| p == path);
    match (add, present) {
        (true, true) => bail!("'{path}' is already included"),
        (false, false) => bail!("'{path}' is not included"),
        (true, false) => list.push(path.to_string()),
        (false, true) => list.retain(|p| p != path),
    }
    if list.is_empty() {
        doc.remove("include");
    } else {
        doc["include"] = Item::Value(str_array(&list));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn via_is_written_only_when_set() {
        let mut r = RuleSpec {
            from: "ui".into(),
            to: "application".into(),
            via: vec!["ports-in".into()],
            action: "allow".into(),
            ..RuleSpec::default()
        };
        let text = rule_table(&r).to_string();
        assert!(text.contains("via = [\"ports-in\"]"), "{text}");
        r.via.clear();
        assert!(!rule_table(&r).to_string().contains("via"));
    }
}
