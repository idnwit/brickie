//! Summary and sum table of the overview.

use crate::state::*;
use brickie_web_api::*;
use std::collections::BTreeMap;
use sycamore::prelude::*;

use super::super::GROUPS;
/// `sumTable`: findings by group and level, counting the enabled layers.
pub(super) fn sum_table(cx: &Cx, scope: Option<&std::collections::BTreeSet<usize>>) -> View {
    let g = &cx.m.g;
    let rows: Vec<View> = GROUPS
        .iter()
        .map(|(grp, name)| {
            let mut n = [0usize; 3]; // error, warn, open
            for (i, d) in cx.m.diags.iter().enumerate() {
                let inside = scope.is_none_or(|s| {
                    d.bricks
                        .iter()
                        .filter_map(|b| g.bricks.iter().position(|x| x.name == *b))
                        .any(|b| s.contains(&b))
                });
                if group_sub(&d.code).0 != *grp || !inside || !cx.eval.with(|ev| ev.von[i]) {
                    continue;
                }
                n[usize::from(d.level != "error")] += 1;
                if !cx.m.cards[i].accepted.get() {
                    n[2] += 1;
                }
            }
            let cells: Vec<View> = n
                .iter()
                .map(|v| {
                    let v = v.to_string();
                    view! { td { (v) } }
                })
                .collect();
            let cells = View::from(cells);
            let cls = format!("gi {grp}");
            let name = name.to_string();
            let icon = match *grp {
                "deps" => "\u{21C4}",
                "cx" => "\u{25A6}",
                _ => "\u{23F1}",
            };
            view! { tr { td { span(class=cls) { (icon) } " " (name) } (cells) } }
        })
        .collect();
    let rows = View::from(rows);
    view! { table(class="sum") { tr { th { "group" } th { "error" } th { "warn" } th { "open" } } (rows) } }
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
