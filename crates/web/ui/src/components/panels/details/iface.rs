//! The Interface section of a brick (declared items, their uses, unused marked) and the
//! reference sites behind a selected edge, both fetched from the server on selection.

use super::super::super::call::call;
use crate::state::*;
use brickie_web_api::*;
use sycamore::prelude::*;

/// A link opening the source popup at a site.
fn site_link(cx: &Cx, s: &SiteView) -> View {
    let href = source_href(
        &s.file,
        s.line,
        Some(s.col),
        Some(s.end_col.saturating_sub(s.col).max(1)),
    );
    let loc = format!("{}:{}", s.file, s.line);
    let link = src_link(cx.modal, href, loc, "mono");
    let written = format!("`{}`", s.written);
    let test = (s.scope == "test").then(|| view! { span(class="tag nm") { "test" } });
    let kind = (s.kind != "uses").then(|| {
        let k = s.kind.clone();
        view! { span(class="tag nm") { (k) } }
    });
    view! { div(class="site") { (link) " " span(class="muted small") { (written) } (test) (kind) } }
}

fn def_link(cx: &Cx, file: &str, line: u32, sig: &str) -> View {
    let href = source_href(file, line, None, None);
    let text = if sig.is_empty() {
        format!("{file}:{line}")
    } else {
        sig.to_string()
    };
    src_link(cx.modal, href, text, "mono sig")
}

fn item_view(cx: &Cx, it: &IfaceItemView, short: &dyn Fn(&str) -> String) -> View {
    let head = format!("{} {}", it.kind, it.name);
    let badge = if it.uses.is_empty() {
        view! { span(class="pill warn", title="no use outside the brick and no test") { "\u{26A0} unused" } }
    } else {
        let n = format!("{} uses", it.use_count());
        view! { span(class="muted small") { (n) } }
    };
    let doc = it.doc.clone();
    let sig = def_link(cx, &it.file, it.line, &it.signature);
    let groups: Vec<View> = it
        .uses
        .iter()
        .map(|g| {
            let name = format!("{} ({})", short(&g.brick), g.sites.len());
            let sites: Vec<View> = g.sites.iter().map(|s| site_link(cx, s)).collect();
            let sites = View::from(sites);
            view! { div(class="ug") { b { (name) } (sites) } }
        })
        .collect();
    let groups = View::from(groups);
    view! {
        details(class="iface") {
            summary { span(class="mono") { (head) } " " (badge) div(class="muted small doc") { (doc) } }
            div { (sig) }
            (groups)
        }
    }
}

/// `root/web/server` -> `web/server`.
fn strip_root(p: &str) -> String {
    p.strip_prefix("root/").unwrap_or(p).to_string()
}

/// The Interface section of the brick at `path`.
pub(super) fn interface_section(cx: &Cx, path: String) -> View {
    let data: Signal<Option<Result<InterfaceView, String>>> = create_signal(None);
    call::<Interface>(&InterfaceRequest { brick: path }, move |r| {
        data.set(Some(r))
    });
    let cx = cx.clone();
    let body = move || match data.get_clone() {
        None => view! { div(class="muted small") { "loading\u{2026}" } },
        Some(Err(e)) => view! { div(class="muted small") { (e) } },
        Some(Ok(i)) => {
            let head = format!(
                "{} \u{b7} {} items \u{b7} {} unused",
                strip_root(&i.brick),
                i.items.len(),
                i.unused()
            );
            let items: Vec<View> = i
                .items
                .iter()
                .map(|it| item_view(&cx, it, &strip_root))
                .collect();
            let items = View::from(items);
            view! { div(class="muted small") { (head) } (items) }
        }
    };
    view! {
        h4(title="What code outside the brick can name, and who uses it (tests anywhere count). Unused items are interface/unused warnings.") { "Interface" }
        (body)
    }
}

fn edge_item_view(cx: &Cx, it: &EdgeItemView) -> View {
    let kind = if it.kind.is_empty() {
        "item".to_string()
    } else {
        it.kind.clone()
    };
    let head = format!("{kind} {} ({})", it.name, it.sites.len());
    let def = match (&it.file, it.line) {
        (Some(f), Some(l)) => def_link(cx, f, l, &it.signature),
        _ => {
            let m = format!("in {}", it.module);
            view! { span(class="muted small") { (m) } }
        }
    };
    let sites: Vec<View> = it.sites.iter().map(|s| site_link(cx, s)).collect();
    let sites = View::from(sites);
    view! {
        details(class="iface") {
            summary { span(class="mono") { (head) } }
            div { (def) }
            (sites)
        }
    }
}

/// Every reference site from the brick at `from` into the brick at `to`, by used item.
pub(super) fn edge_section(cx: &Cx, from: String, to: String) -> View {
    let data: Signal<Option<Result<Vec<EdgeItemView>, String>>> = create_signal(None);
    call::<EdgeUses>(&EdgeUsesRequest { from, to }, move |r| data.set(Some(r)));
    let cx = cx.clone();
    let body = move || match data.get_clone() {
        None => view! { div(class="muted small") { "loading\u{2026}" } },
        Some(Err(e)) => view! { div(class="muted small") { (e) } },
        Some(Ok(v)) => {
            let n: usize = v.iter().map(|i| i.sites.len()).sum();
            let head = format!("{} items \u{b7} {n} sites", v.len());
            let items: Vec<View> = v.iter().map(|it| edge_item_view(&cx, it)).collect();
            let items = View::from(items);
            view! { div(class="muted small") { (head) } (items) }
        }
    };
    view! {
        h4(title="Every reference that makes up this edge, grouped by the used item of the target") { "Code behind the edge" }
        (body)
    }
}
