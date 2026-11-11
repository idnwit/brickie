//! The right panel (`renderDetail`): overview, brick, group, edge or finding details, then the
//! scoped functions table and the legend.

use super::{GROUPS, card::card_view, functions::functions_view, legend::legend};
use crate::model::*;
use crate::state::*;
use std::collections::BTreeMap;
use sycamore::prelude::*;

pub fn brick_link(cx: &Cx, b: usize) -> View {
    let name = cx.m.g.bricks[b].name.clone();
    let href = Sel::Brick(b).apply(&cx.params.get_clone(), &cx.m.g).url();
    let c = cx.clone();
    view! { a(href=href, on:click=move |e: Click| { e.prevent_default(); c.select(Sel::Brick(b)); }) { (name) } }
}

pub fn edge_link(cx: &Cx, e: usize, other: usize) -> View {
    let x = cx.m.g.edges[e].clone();
    let href = Sel::Edge(e).apply(&cx.params.get_clone(), &cx.m.g).url();
    let c = cx.clone();
    let label = cx.m.g.bricks[other].name.clone();
    let tag = format!("tag{}", if x.diags.is_empty() { "" } else { " debt" });
    let note = if x.test { " (tests only)" } else { "" };
    view! {
        div { a(href=href, on:click=move |ev: Click| { ev.prevent_default(); c.select(Sel::Edge(e)); }) {
            span(class=tag) { (x.kind.clone()) } (label)
        } span(class="muted") { (note) } }
    }
}

/// `sumTable`: findings by group and level, counting the enabled layers.
pub fn sum_table(cx: &Cx, scope: Option<&std::collections::BTreeSet<usize>>) -> View {
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

pub fn summary(cx: &Cx) -> View {
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

pub fn brick_detail(cx: &Cx, b: usize) -> View {
    let br = cx.m.g.bricks[b].clone();
    let g = &cx.m.g;
    let out: Vec<View> = (0..g.edges.len())
        .filter(|&e| g.edges[e].from == b)
        .map(|e| edge_link(cx, e, g.edges[e].to))
        .collect();
    let inn: Vec<View> = (0..g.edges.len())
        .filter(|&e| g.edges[e].to == b)
        .map(|e| edge_link(cx, e, g.edges[e].from))
        .collect();
    let name = br.name.clone();
    let (no, ni) = (out.len(), inn.len());
    let (out, inn) = (View::from(out), View::from(inn));
    let tags: Vec<View> = br
        .tags
        .iter()
        .cloned()
        .map(|t| view! { span(class="tag") { (t) } })
        .collect();
    let tags = View::from(tags);
    let members: Vec<View> = br
        .members
        .iter()
        .cloned()
        .map(|m| view! { div(class="mono") { (m) } })
        .collect();
    let members = View::from(members);
    let card = scorecard(cx, b);
    view! {
        h2 { (name) }
        div { (tags) }
        (card)
        h4 { "Members" }
        (members)
        h4 { (format!("Depends on ({no})")) }
        (out)
        h4 { (format!("Used by ({ni})")) }
        (inn)
    }
}

/// The scorecard of a brick: score, open findings per group, worst functions, edge counts.
fn scorecard(cx: &Cx, b: usize) -> View {
    let s = cx.scores.with(|s| s[b].clone());
    let brick = &cx.m.g.bricks[b];
    let badge = match (s.errors, s.warns) {
        (0, 0) => view! { span(class="pill clean") { "\u{2713} clean" } },
        (e, 0) => view! { span(class="pill") { (format!("{e}E")) } },
        (0, w) => view! { span(class="pill warn") { (format!("{w}W")) } },
        (e, w) => view! { span(class="pill") { (format!("{e}E {w}W")) } },
    };
    let rows: Vec<View> = CATEGORIES
        .iter()
        .zip(s.by_cat)
        .map(|(name, n)| {
            let (name, n) = (name.to_string(), n.to_string());
            view! { tr { td { (name) } td { (n) } } }
        })
        .collect();
    let rows = View::from(rows);
    let fns: Vec<View> = brick
        .hot
        .iter()
        .take(3)
        .map(|f| {
            let link = src_link(
                cx.modal,
                source_href(&f.file, f.line, None, None),
                f.qname.clone(),
                "mono",
            );
            let m = format!("{} {}/{}", f.metric, f.value, f.limit);
            let r = format!("{}\u{00D7}", fmt_score(f.ratio));
            view! { div(class="hf") { (link) span(class="muted small") { (m) } b { (r) } } }
        })
        .collect();
    let fns = if fns.is_empty() {
        view! { div(class="muted small") { "no function over its limit" } }
    } else {
        View::from(fns)
    };
    let edges = format!(
        "in {} ({} violating) \u{00B7} out {} ({} violating)",
        s.edges_in, s.viol_in, s.edges_out, s.viol_out
    );
    let score = fmt_score(s.score);
    let worst = format!(
        "worst function at {}\u{00D7} its limit",
        fmt_score(brick.worst_ratio)
    );
    view! {
        div(class="scorecard") {
            div(class="big", title="score = open errors + 0.25 x open warnings + complexity excess (any layer). A cycle counts once for each brick in it.") { span { (score) } small { "score" } (badge) }
            div(class="muted small") { (worst) }
            table { tr { th { "open findings" } th { "n" } } (rows) }
            h4 { "Worst functions" }
            (fns)
            h4 { "Edges" }
            div(class="small", title="Edges to and from this brick; violating = the edge carries an open deps finding (a deny or default rule, or a cycle through it)") { (edges) }
        }
    }
}

pub fn group_detail(cx: &Cx) -> View {
    let g = &cx.m.g;
    let (set, name) = cx.eval.with(|ev| (ev.sel.set(g), ev.sel.name(g)));
    let mut cnt: [BTreeMap<String, usize>; 3] = Default::default();
    for (e, x) in g.edges.iter().enumerate() {
        if cx.eval.with(|ev| ev.edge[e].is_none()) {
            continue;
        }
        let k = match (set.contains(&x.from), set.contains(&x.to)) {
            (false, true) => 0,
            (true, false) => 1,
            (true, true) => 2,
            _ => continue,
        };
        *cnt[k].entry(x.kind.clone()).or_default() += 1;
    }
    let by = |m: &BTreeMap<String, usize>| {
        let v: Vec<View> = m
            .iter()
            .map(|(k, n)| {
                let t = format!("{k} {n}");
                view! { span(class="tag") { (t) } }
            })
            .collect();
        if v.is_empty() {
            view! { span(class="muted") { "0" } }
        } else {
            View::from(v)
        }
    };
    let members: Vec<View> = set
        .iter()
        .map(|&b| {
            let l = brick_link(cx, b);
            view! { div { (l) } }
        })
        .collect();
    let members = View::from(members);
    let table = sum_table(cx, Some(&set));
    let (a, b, c) = (by(&cnt[0]), by(&cnt[1]), by(&cnt[2]));
    view! {
        h2 { (name) }
        p(class="muted") { "Selected group (Shift+click adds or removes, Esc clears). Only edges with an end in the group are drawn; the left list shows findings that touch it." }
        h4 { (format!("Bricks ({})", set.len())) }
        (members)
        h4 { "Into the group" } div { (a) }
        h4 { "Out of the group" } div { (b) }
        h4 { "Inside" } div { (c) }
        h4 { "Findings in scope" }
        (table)
    }
}

pub fn edge_detail(cx: &Cx, e: usize) -> View {
    let g = &cx.m.g;
    let x = g.edges[e].clone();
    let finds: Vec<View> = x
        .diags
        .iter()
        .cloned()
        .map(|id| {
            let href = cx.params.get_clone().with(|p| p.item = id.clone()).url();
            let (c, id2) = (cx.clone(), id.clone());
            view! { div { a(href=href, class="mono", on:click=move |ev: Click| {
                ev.prevent_default();
                c.go(c.params.get_clone().with(|p| p.item = id2.clone()));
            }) { (id) } } }
        })
        .collect();
    let finds = View::from(finds);
    let scope = if x.test { "test scope only" } else { "prod" };
    let (from, to, kind) = (brick_link(cx, x.from), brick_link(cx, x.to), x.kind.clone());
    let title = format!("Deps findings ({})", x.diags.len());
    view! {
        h2 { (from) " " span(class="muted") { (kind) } " \u{2192} " (to) }
        div { span(class="tag") { (scope) } }
        h4 { (title) }
        (finds)
    }
}

pub fn finding_detail(cx: &Cx, i: usize) -> View {
    let card = cx.m.cards[i].clone();
    let bricks: Vec<View> = card
        .d
        .bricks
        .iter()
        .filter_map(|n| cx.brick(n))
        .map(|b| {
            let l = brick_link(cx, b);
            view! { span { (l) " " } }
        })
        .collect();
    let bricks = View::from(bricks);
    let c = cx.clone();
    let clear = view! { a(href="#", on:click=move |e: Click| { e.prevent_default(); c.go(c.params.get_clone().with(|p| p.item.clear())); }) { "\u{2190} back" } };
    let card_v = card_view(card, cx.modal);
    view! {
        div(class="sh") { (clear) }
        p { "Bricks: " (bricks) }
        (card_v)
    }
}

/// `renderDetail`.
pub fn detail(cx: &Cx) -> View {
    let content = {
        let cx = cx.clone();
        move || {
            let item = cx.params.with(|p| p.item.clone());
            if let Some(i) =
                cx.m.diags
                    .iter()
                    .position(|d| d.id == item)
                    .filter(|_| !item.is_empty())
            {
                return finding_detail(&cx, i);
            }
            match cx.eval.with(|ev| ev.sel.clone()) {
                Sel::None => summary(&cx),
                Sel::Brick(b) => brick_detail(&cx, b),
                Sel::Group { .. } => group_detail(&cx),
                Sel::Edge(e) => edge_detail(&cx, e),
            }
        }
    };
    let fns = functions_view(cx);
    let leg = legend(cx);
    view! {
        aside(id="detail") {
            div(id="dcont") { (content) (fns) }
            (leg)
        }
    }
}
