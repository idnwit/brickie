//! Summary and sum table of the overview.

use crate::state::*;
use brickie_web_api::*;
use std::collections::BTreeMap;
use sycamore::prelude::*;

use super::super::GROUPS;
use super::super::diagnostics::gi_icon;

/// `sumTable`: findings by group, counting the enabled layers.
pub(super) fn sum_table(cx: &Cx) -> View {
    let rows: Vec<View> = GROUPS
        .iter()
        .map(|(grp, name)| sum_row(cx, grp, name))
        .collect();
    let rows = View::from(rows);
    view! { table(class="sum") { tr { th { "group" } th { "findings" } } (rows) } }
}

/// One group row: icon, name and the count.
fn sum_row(cx: &Cx, grp: &str, name: &str) -> View {
    let n = group_count(cx, grp).to_string();
    let cls = format!("gi {grp}");
    let (name, icon) = (name.to_string(), gi_icon(grp));
    view! { tr { td { span(class=cls) { (icon) } " " (name) } td { (n) } } }
}

/// Count of the shown findings of `grp`.
fn group_count(cx: &Cx, grp: &str) -> usize {
    cx.m.diags
        .iter()
        .enumerate()
        .filter(|(i, d)| group_sub(&d.code).0 == grp && cx.eval.with(|ev| ev.von[*i]))
        .count()
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
    let table = sum_table(cx);
    let lv = cx.level();
    let lines = (lv.code_lines + lv.test_lines > 0).then(|| {
        let s = fmt_lines_detail(lv.code_lines, lv.test_lines);
        view! { p(class="muted small") { (s) } }
    });
    let title = format!("Findings by group (layers on: {layers_on}/{total})");
    view! {
        h2 { "Overview" }
        p(class="muted") { "Click a brick or an edge to scope the lists. Click a finding for details. Keys: 1 2 3 presets, j/k findings, Esc clear, / filter." }
        (lines)
        h4 { (title) }
        (table)
        div(class="tagrow") { (tags) }
    }
}
