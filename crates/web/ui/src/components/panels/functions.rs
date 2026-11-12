//! The functions table scoped to the selection.

use crate::state::*;
use brickie_web_api::*;
use sycamore::prelude::*;

/// The functions table scoped to the selection (`Functions (n)`).
pub fn functions_view(cx: &Cx) -> View {
    let mut scope: Vec<View> = Vec::new();
    for s in ["prod", "test", "all"] {
        scope.push(nav_link(
            cx,
            s.into(),
            "chip",
            move |p| p.scope() == s,
            move |p| p.with(|p| p.fn_scope = s.to_string()),
        ));
    }
    let scope = View::from(scope);
    let mut heads: Vec<View> = Vec::new();
    for m in std::iter::once("name").chain(METRICS) {
        let label = m.to_string();
        let active = move |p: &Params| p.sort() == m;
        let change = move |p: &Params| {
            p.with(|p| {
                if p.sort() == m {
                    let asc = if m == "name" {
                        p.fn_dir == "desc"
                    } else {
                        p.desc()
                    };
                    p.fn_dir = if asc { "asc" } else { "desc" }.into();
                } else {
                    p.fn_sort = m.to_string();
                    p.fn_dir = String::new();
                }
            })
        };
        let cell = nav_link(cx, label, "sort", active, change);
        heads.push(view! { th { (cell) } });
    }
    let heads = View::from(heads);
    // Functions of the selected bricks (by tree path; no selection: the whole level), through
    // the same filter the JSON API uses.
    let scoped = {
        let cx = cx.clone();
        move || {
            let p = cx.params.get_clone();
            let names: Option<Vec<String>> = cx.eval.with(|ev| ev.sel.scope(&cx.m.g)).map(|s| {
                s.into_iter()
                    .map(|b| cx.m.g.bricks[b].path.clone())
                    .collect()
            });
            let level = cx.level().path.clone();
            match names {
                Some(n) => p.with(|p| p.brick = n.join(",")),
                None if level == "root" => p.with(|p| p.brick.clear()),
                None => p.with(|p| p.brick = level),
            }
        }
    };
    let rows = {
        let (cx, scoped) = (cx.clone(), scoped.clone());
        move || {
            let p = scoped();
            let all = &cx.m.fns;
            let idx = fn_rows(all, &p);
            let total = idx.len();
            let shown = p.limit().map_or(total, |l| l.min(total));
            let mut out: Vec<View> = idx[..shown]
                .iter()
                .map(|&i| {
                    let f = &all[i];
                    let name = src_link(
                        cx.modal,
                        source_href(&f.file, f.line, None, None),
                        f.qname.clone(),
                        "mono",
                    );
                    let brick = f
                        .brick
                        .strip_prefix("root/")
                        .unwrap_or(&f.brick)
                        .to_string();
                    let cells: Vec<View> = METRICS
                        .iter()
                        .map(|m| f.metric(m).to_string())
                        .map(|c| view! { td { (c) } })
                        .collect();
                    let cells = View::from(cells);
                    view! { tr { td { (name) } (cells) td(class="brick muted") { (brick) } } }
                })
                .collect();
            if shown < total {
                let more = nav_link(
                    &cx,
                    format!("show all {total} functions"),
                    "chip",
                    |_| false,
                    |p| p.with(|p| p.fn_limit = "all".into()),
                );
                out.push(view! { tr { td(colspan="8") { (more) } } });
            }
            if total == 0 {
                out.push(view! { tr { td(colspan="8", class="muted") { "No functions match." } } });
            }
            View::from(out)
        }
    };
    let title = {
        let (cx, scoped) = (cx.clone(), scoped);
        move || format!("Functions ({})", fn_rows(&cx.m.fns, &scoped()).len())
    };
    view! {
        div(id="functions") {
            h4 { (title) }
            div(class="chips") { span(class="label") { "scope" } (scope) }
            table(class="fns") {
                thead { tr { (heads) th(class="brick") { "brick" } } }
                tbody { (rows) }
            }
        }
    }
}
