//! Brick detail and its scorecard.

use super::links::edge_link;
use crate::state::*;
use brickie_web_api::*;
use sycamore::prelude::*;

pub(super) fn brick_detail(cx: &Cx, b: usize) -> View {
    let br = cx.m.g.bricks[b].clone();
    let g = &cx.m.g;
    let out: Vec<View> = (0..g.edges.len())
        .filter(|&e| g.edges[e].from == b && !g.edges[e].frame())
        .map(|e| edge_link(cx, e, g.edges[e].to))
        .collect();
    let inn: Vec<View> = (0..g.edges.len())
        .filter(|&e| g.edges[e].to == b && !g.edges[e].frame())
        .map(|e| edge_link(cx, e, g.edges[e].from))
        .collect();
    let name = br
        .name
        .strip_prefix("root/")
        .unwrap_or(&br.name)
        .to_string();
    let open_code = src_link(
        cx.modal,
        code_brick_href(&br.name),
        "</> Code".into(),
        "codebtn",
    );
    let (no, ni) = (out.len(), inn.len());
    let (out, inn) = (View::from(out), View::from(inn));
    let tags: Vec<View> = br
        .tags
        .iter()
        .cloned()
        .map(|t| view! { span(class="tag") { (t) } })
        .collect();
    let tags = View::from(tags);
    let desc = super::super::node::desc(&br);
    let members = super::super::node::tree_info(cx, &br);
    let card = scorecard(cx, b);
    let iface = super::iface::interface_section(cx, br.path.clone());
    view! {
        div(class="dhead") { h2 { (name) } span(title="Open this brick's code") { (open_code) } }
        (desc)
        div { (tags) }
        (card)
        (members)
        h4 { (format!("Depends on ({no})")) }
        (out)
        h4 { (format!("Used by ({ni})")) }
        (inn)
        (iface)
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
                code_href(&f.file, f.line, None),
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
