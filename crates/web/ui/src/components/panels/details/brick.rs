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
    let is_local_group = br.is_group && !br.ghost;
    let header_links = if is_local_group {
        View::default()
    } else {
        let open_code = src_link(
            cx.modal,
            code_brick_href(&br.name),
            "</> Code".into(),
            "codebtn",
        );
        let docs_url = docs_href(&br.name);
        view! {
            span(title="Open this brick's code") { (open_code) }
            a(class="codebtn", href=docs_url, target="_blank", title="Open this brick's documentation page") { "Docs" }
        }
    };
    let (no, ni) = (out.len(), inn.len());
    let (out, inn) = (View::from(out), View::from(inn));
    let tags: Vec<View> = br
        .tags
        .iter()
        .cloned()
        .map(|t| view! { span(class="tag") { (t) } })
        .collect();
    let tags = View::from(tags);
    let used_by = format!("Used by ({ni})");
    let hidden = if br.independent && br.hidden_in > 0 {
        let note = format!(
            "used by {} more brick(s); edges hidden: independent lib, no rule violated",
            br.hidden_in
        );
        view! { div(class="muted small", title=INDEPENDENT_TIP) { (note) } }
    } else {
        View::default()
    };
    let mark = if br.independent {
        view! { span(class="tag", title=INDEPENDENT_TIP) { (INDEPENDENT_LABEL) } }
    } else {
        View::default()
    };
    let desc = super::super::node::desc(&br);
    let members = if is_local_group {
        View::default()
    } else {
        super::super::node::tree_info(cx, &br)
    };
    let group_list = if is_local_group {
        group_members_section(cx, b, &br)
    } else {
        View::default()
    };
    let card = scorecard(cx, b);
    let iface = if br.is_group {
        View::default()
    } else {
        super::iface::interface_section(cx, br.path.clone())
    };
    view! {
        div(class="dhead") {
            h2 { (name) }
            (header_links)
        }
        (desc)
        div { (tags) (mark) }
        (card)
        (group_list)
        (members)
        h4 { (format!("Depends on ({no})")) }
        (out)
        h4 { (used_by) }
        (hidden)
        (inn)
        (iface)
    }
}

/// Detail panel for a single member brick inside a peer group.
pub(super) fn member_detail(cx: &Cx, b: usize, m: usize) -> View {
    let grp = cx.m.g.bricks[b].clone();
    let mem = grp.group_members[m].clone();
    let open_code = src_link(
        cx.modal,
        code_brick_href(&mem.path),
        "</> Code".into(),
        "codebtn",
    );
    let grp_href = Sel::Brick(b).apply(&cx.params.get_clone(), &cx.m.g).url();
    let c = cx.clone();
    let grp_name = grp.name.clone();
    let zoom = mem.is_composite.then(|| {
        let c = cx.clone();
        let target = mem.path.clone();
        view! { button(on:click=move |_: Click| c.go(c.params.with(|p| p.at_level(&target)))) { "Zoom in" } }
    });
    let badge = match mem.errors {
        0 => view! { span(class="pill clean") { "\u{2713} clean" } },
        e => view! { span(class="pill") { (format!("{e}E")) } },
    };
    let score = fmt_score(mem.score);
    let worst = format!(
        "worst function at {}\u{00D7} its limit",
        fmt_score(mem.worst_ratio)
    );
    let iface = super::iface::interface_section(cx, mem.path.clone());
    let mname = mem.name.clone();
    let docs_url = docs_href(&mem.path);
    view! {
        div(class="dhead") {
            h2 { (mname) }
            span(title="Open this brick's code") { (open_code) }
            a(class="codebtn", href=docs_url, target="_blank", title="Open this brick's documentation page") { "Docs" }
        }
        p(class="muted") {
            "in group "
            a(href=grp_href, on:click=move |e: Click| { e.prevent_default(); c.select(Sel::Brick(b)); }) { (grp_name) }
        }
        (zoom)
        div(class="scorecard") {
            div(class="big") { span { (score) } small { "score" } (badge) }
            div(class="muted small") { (worst) }
        }
        (iface)
    }
}

fn group_members_section(cx: &Cx, b: usize, br: &BrickView) -> View {
    let items: Vec<View> = br
        .group_members
        .iter()
        .enumerate()
        .map(|(mi, m)| {
            let href = Sel::Member(b, mi)
                .apply(&cx.params.get_clone(), &cx.m.g)
                .url();
            let c = cx.clone();
            let name = m.name.clone();
            let err_label = format!("{}E", m.errors);
            let err = (m.errors > 0).then(move || view! { span(class="pill") { (err_label) } });
            view! {
                div {
                    a(href=href, on:click=move |e: Click| { e.prevent_default(); c.select(Sel::Member(b, mi)); }) { (name) }
                    " " (err)
                }
            }
        })
        .collect();
    let count = items.len();
    let items = if items.is_empty() {
        view! { div(class="muted small") { "no member bricks yet" } }
    } else {
        View::from(items)
    };
    let internal = if br.internal_edges.is_empty() {
        View::default()
    } else {
        let rows: Vec<View> = br
            .internal_edges
            .iter()
            .map(|ie| {
                let text = format!(
                    "{} \u{2192} {} ({}, {} ref(s))",
                    ie.from_member, ie.to_member, ie.kind, ie.count
                );
                view! { div(class="err-text small") { (text) } }
            })
            .collect();
        let n = rows.len();
        let rows = View::from(rows);
        view! { h4 { (format!("Intra-group violations ({n})")) } (rows) }
    };
    view! {
        h4 { (format!("Group members ({count})")) }
        (items)
        (internal)
    }
}

/// The scorecard of a brick: score, findings per group, worst functions, edge counts.
fn scorecard(cx: &Cx, b: usize) -> View {
    let s = cx.scores.with(|s| s[b].clone());
    let brick = &cx.m.g.bricks[b];
    let badge = match s.errors {
        0 => view! { span(class="pill clean") { "\u{2713} clean" } },
        e => view! { span(class="pill") { (format!("{e}E")) } },
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
    let edges = if s.viol_internal > 0 {
        format!(
            "in {} ({} violating) \u{00B7} out {} ({} violating) \u{00B7} {} internal violating",
            s.edges_in, s.viol_in, s.edges_out, s.viol_out, s.viol_internal
        )
    } else {
        format!(
            "in {} ({} violating) \u{00B7} out {} ({} violating)",
            s.edges_in, s.viol_in, s.edges_out, s.viol_out
        )
    };
    let score = fmt_score(s.score);
    let worst = format!(
        "worst function at {}\u{00D7} its limit",
        fmt_score(brick.worst_ratio)
    );
    view! {
        div(class="scorecard") {
            div(class="big", title="score = findings + complexity excess (any layer). A cycle counts once for each brick in it.") { span { (score) } small { "score" } (badge) }
            div(class="muted small") { (worst) }
            table { tr { th { "findings" } th { "n" } } (rows) }
            h4 { "Worst functions" }
            (fns)
            h4 { "Edges" }
            div(class="small", title="Edges to and from this brick; violating = the edge carries a deps finding (no link allows it, a private target, or a cycle through it)") { (edges) }
        }
    }
}
