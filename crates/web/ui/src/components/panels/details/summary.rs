//! Summary and sum table of the overview.

use crate::state::*;
use brickie_web_api::*;
use std::collections::BTreeMap;
use sycamore::prelude::*;

use super::super::GROUPS;
use super::super::diagnostics::gi_icon;
use std::collections::BTreeSet;

/// `sumTable`: findings by group and level, counting the enabled layers.
pub(super) fn sum_table(cx: &Cx, scope: Option<&BTreeSet<usize>>) -> View {
    let rows: Vec<View> = GROUPS
        .iter()
        .map(|(grp, name)| sum_row(cx, grp, name, scope))
        .collect();
    let rows = View::from(rows);
    view! { table(class="sum") { tr { th { "group" } th { "error" } th { "warn" } th { "open" } } (rows) } }
}

/// One group row: icon, name and the error / warn / open counts.
fn sum_row(cx: &Cx, grp: &str, name: &str, scope: Option<&BTreeSet<usize>>) -> View {
    let cells: Vec<View> = group_counts(cx, grp, scope)
        .iter()
        .map(|v| {
            let v = v.to_string();
            view! { td { (v) } }
        })
        .collect();
    let cells = View::from(cells);
    let cls = format!("gi {grp}");
    let (name, icon) = (name.to_string(), gi_icon(grp));
    view! { tr { td { span(class=cls) { (icon) } " " (name) } (cells) } }
}

/// Counts of the shown findings of `grp` in `scope`: errors, warnings and open ones.
fn group_counts(cx: &Cx, grp: &str, scope: Option<&BTreeSet<usize>>) -> [usize; 3] {
    let mut n = [0usize; 3]; // error, warn, open
    for (i, d) in cx.m.diags.iter().enumerate() {
        if group_sub(&d.code).0 != grp
            || !in_scope(&cx.m.g, d, scope)
            || !cx.eval.with(|ev| ev.von[i])
        {
            continue;
        }
        n[usize::from(d.level != "error")] += 1;
        if !cx.m.cards[i].accepted.get() {
            n[2] += 1;
        }
    }
    n
}

/// Whether finding `d` touches a brick of `scope` (no scope: every finding does).
fn in_scope(g: &GraphView, d: &DiagView, scope: Option<&BTreeSet<usize>>) -> bool {
    scope.is_none_or(|s| {
        d.bricks
            .iter()
            .filter_map(|b| g.bricks.iter().position(|x| x.name == *b))
            .any(|b| s.contains(&b))
    })
}

pub(super) fn summary(cx: &Cx) -> View {
    let layers_on = cx.eval.with(|ev| ev.von.iter().filter(|v| **v).count());
    let total = cx.m.diags.len();
    let mut subs: BTreeMap<String, usize> = BTreeMap::new();
    for d in &cx.m.diags {
        *subs.entry(d.code.clone()).or_default() += 1;
    }
    let tags: Vec<View> = subs
        .into_iter()
        .map(|(k, n)| view! { span(class="tag") { (format!("{k} {n}")) } })
        .collect();
    let tags = View::from(tags);
    let table = sum_table(cx, None);
    let title = format!("Findings by group and level (layers on: {layers_on}/{total})");
    view! {
        h2 { "Overview" }
        p(class="muted") { "Click a brick, an edge or a tag row to scope the lists (Shift: add). Click a finding for details. Keys: 1 2 3 presets, j/k findings, Esc clear, / filter." }
        h4 { (title) }
        (table)
        div(class="tagrow") { (tags) }
    }
}
