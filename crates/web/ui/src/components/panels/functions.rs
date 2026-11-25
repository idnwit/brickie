//! The functions table scoped to the selected brick (hidden while a link is selected).

use crate::state::*;
use brickie_web_api::*;
use sycamore::prelude::*;

/// The functions table scoped to the selection (`Functions (n)`).
pub fn functions_view(cx: &Cx) -> View {
    let scope = scope_chips(cx);
    let heads = sort_heads(cx);
    let rows = {
        let cx = cx.clone();
        move || fn_table_rows(&cx)
    };
    let title = {
        let cx = cx.clone();
        move || format!("Functions ({})", fn_rows(&cx.m.fns, &scoped(&cx)).len())
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

/// The `prod` / `test` / `all` chips.
fn scope_chips(cx: &Cx) -> View {
    let chips: Vec<View> = ["prod", "test", "all"]
        .into_iter()
        .map(|s| {
            nav_link(
                cx,
                s.into(),
                "chip",
                move |p| p.scope() == s,
                move |p| p.with(|p| p.fn_scope = s.to_string()),
            )
        })
        .collect();
    View::from(chips)
}

/// The sortable header cells: the name and each metric.
fn sort_heads(cx: &Cx) -> View {
    let heads: Vec<View> = std::iter::once("name")
        .chain(METRICS)
        .map(|m| {
            let cell = nav_link(
                cx,
                m.to_string(),
                "sort",
                move |p: &Params| p.sort() == m,
                move |p: &Params| resort(p, m),
            );
            view! { th { (cell) } }
        })
        .collect();
    View::from(heads)
}

/// The params after a click on the header of metric `m`: the active column flips its
/// direction (the name column is ascending first), another column becomes the sort.
fn resort(p: &Params, m: &str) -> Params {
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
}

/// Functions of the selected brick or member (by tree path; no selection: the whole level),
/// through the same filter the JSON API uses.
fn scoped(cx: &Cx) -> Params {
    let p = cx.params.get_clone();
    let name = cx.eval.with(|ev| match ev.sel {
        Sel::Brick(b) => {
            let br = &cx.m.g.bricks[b];
            if br.is_group && !br.group_members.is_empty() {
                Some(
                    br.group_members
                        .iter()
                        .map(|m| m.path.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                )
            } else {
                Some(br.path.clone())
            }
        }
        Sel::Member(b, m) => Some(cx.m.g.bricks[b].group_members[m].path.clone()),
        _ => None,
    });
    with_bricks(&p, name, cx.level().path.clone())
}

/// `p` limited to the selected brick `name`, or else to the `level` path (the root
/// level means no limit).
fn with_bricks(p: &Params, name: Option<String>, level: String) -> Params {
    match name {
        Some(n) => p.with(|p| p.brick = n),
        None if level == "root" => p.with(|p| p.brick.clear()),
        None => p.with(|p| p.brick = level),
    }
}

/// The table body: the shown rows, then a "show all" row and a "no match" row as needed.
fn fn_table_rows(cx: &Cx) -> View {
    let p = scoped(cx);
    let all = &cx.m.fns;
    let idx = fn_rows(all, &p);
    let total = idx.len();
    let shown = p.limit().map_or(total, |l| l.min(total));
    let mut out: Vec<View> = idx[..shown].iter().map(|&i| fn_row(cx, &all[i])).collect();
    if shown < total {
        let more = nav_link(
            cx,
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

/// One function: source link, the metrics and its brick.
fn fn_row(cx: &Cx, f: &FnRow) -> View {
    let name = src_link(
        cx.modal,
        code_href(&f.file, f.line, None),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(sort: &str, dir: &str) -> Params {
        Params {
            fn_sort: sort.into(),
            fn_dir: dir.into(),
            ..Default::default()
        }
    }

    #[test]
    fn resort_flips_the_active_metric() {
        // The default sort is cognitive, descending.
        assert_eq!(resort(&Params::default(), "cognitive"), p("", "asc"));
        assert_eq!(resort(&p("", "asc"), "cognitive"), p("", "desc"));
        assert_eq!(resort(&p("lines", "desc"), "lines"), p("lines", "asc"));
    }

    #[test]
    fn resort_name_starts_ascending() {
        assert_eq!(resort(&Params::default(), "lines"), p("lines", ""));
        assert_eq!(resort(&p("name", ""), "name"), p("name", "desc"));
        assert_eq!(resort(&p("name", "desc"), "name"), p("name", "asc"));
    }

    #[test]
    fn bricks_follow_the_selection_then_the_level() {
        let base = Params {
            brick: "old".into(),
            ..Default::default()
        };
        let sel = Some("root/a".to_string());
        assert_eq!(with_bricks(&base, sel, "root/x".into()).brick, "root/a");
        assert_eq!(with_bricks(&base, None, "root".into()).brick, "");
        assert_eq!(with_bricks(&base, None, "root/x".into()).brick, "root/x");
    }
}
