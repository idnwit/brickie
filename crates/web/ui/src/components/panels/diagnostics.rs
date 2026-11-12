//! The left panel (`renderLeft`): scoped findings list (`fRow`, `listFindings`) and filter chips.

use super::GROUPS;
use super::bricks::brick_index;
use crate::state::*;
use brickie_web_api::{Params, group_sub};
use std::collections::BTreeSet;
use sycamore::prelude::*;

pub fn gi_icon(g: &str) -> &'static str {
    match g {
        "deps" => "\u{21C4}",
        "cx" => "\u{25A6}",
        "iface" => "\u{25C7}",
        _ => "\u{23F1}",
    }
}

/// `fRow`: one finding in the left list.
pub fn finding_row(cx: &Cx, i: usize) -> View {
    let card = cx.m.cards[i].clone();
    let d = card.d.clone();
    let (g, _) = group_sub(&d.code);
    let id = d.id.clone();
    let href = jump(&cx.params.get_clone(), &d).url();
    let (c0, c1, c2, c3) = (cx.clone(), cx.clone(), cx.clone(), cx.clone());
    let acc = card.accepted;
    let cls = move || {
        let sel = c0.params.with(|p| p.item == id);
        format!("fi{}", if sel { " sel" } else { "" })
    };
    let lv = format!("lv {}", d.level);
    let gcls = format!("gi {g}");
    let bricks = d.bricks.join(if d.code == "deps/cycle" {
        " \u{21C4} "
    } else {
        " \u{2192} "
    });
    let dot = move || if acc.get() { "dot accepted" } else { "dot new" };
    let st = move || if acc.get() { "accepted" } else { "open" };
    let tip = format!("{bricks}\n{}", d.message);
    let click = {
        let d1 = d.clone();
        move |e: Click| {
            if plain(&e) {
                e.prevent_default();
                let p = jump(&c1.params.get_clone(), &d1);
                c1.go(p);
            }
        }
    };
    let (level, message) = (d.level.clone(), d.message.clone());
    let at = d
        .brick_level
        .as_ref()
        .map(|l| format!("@ {l}"))
        .unwrap_or_default();
    view! {
        a(class=cls, href=href, title=tip, on:click=click,
            on:mouseenter=move |_: Click| c2.hover.set(Some(i)),
            on:mouseleave=move |_: Click| c3.hover.set(None)) {
            div(class="l1") {
                span(class=gcls) { (gi_icon(g)) }
                span(class=lv) { (level) }
                span(class="bk") { (bricks) }
                span(class="at") { (at) }
                span(class="st") { span(class=dot) {} (st) }
            }
            div(class="l2") { (message) }
        }
    }
}

/// Select finding `d`, at its level: the view jumps to the level that reports it.
fn jump(p: &brickie_web_api::Params, d: &brickie_web_api::DiagView) -> brickie_web_api::Params {
    let mut next = match &d.brick_level {
        Some(l) if l != p.level_path() => p.at_level(l),
        _ => p.clone(),
    };
    next.item = d.id.clone();
    next
}

/// The findings the left list shows: layers, scope, filters and the text box.
pub fn current_list(cx: &Cx) -> Vec<usize> {
    let p = cx.params.get_clone();
    let acc: Vec<bool> = cx.m.cards.iter().map(|c| c.accepted.get()).collect();
    let idx = cx
        .eval
        .with(|ev| list_findings(&cx.m.g, &cx.m.diags, &acc, &p, ev));
    let t = cx.q.get_clone().to_lowercase();
    idx.into_iter()
        .filter(|&i| {
            let d = &cx.m.diags[i];
            t.is_empty()
                || format!("{} {} {}", d.id, d.bricks.join(" "), d.message)
                    .to_lowercase()
                    .contains(&t)
        })
        .collect()
}

/// `renderLeft`.
pub fn left(cx: &Cx) -> View {
    let scope = scope_bar(cx);
    let filters = filter_bar(cx);
    let q = cx.q;
    let list = findings_list(cx);
    let index = brick_index(cx);
    view! {
        nav(id="left") {
            (scope)
            (filters)
            input(id="q", placeholder="filter text ( / ) \u{00B7} j/k to move", bind:value=q)
            (list)
            (index)
        }
    }
}

/// The scope line with a clear button while something is selected.
fn scope_bar(cx: &Cx) -> impl Fn() -> View + 'static {
    let cx = cx.clone();
    move || {
        let (name, has) = cx
            .eval
            .with(|ev| (ev.sel.name(&cx.m.g), ev.sel != Sel::None));
        let clear = if has {
            let c = cx.clone();
            view! { button(title="Clear the scope (Esc)", on:click=move |_: Click| c.select(Sel::None)) { "\u{00D7} clear" } }
        } else {
            View::default()
        };
        view! { div(class="scope") { "Scope: " b { (name) } (clear) } }
    }
}

/// A filter chip: its param `field` and the `value` it sets.
type Chip = (&'static str, String, &'static str);

/// The filter chip rows: group, level, levels, status and finding code.
fn filter_bar(cx: &Cx) -> View {
    let grp = chip_row(
        cx,
        "group",
        std::iter::once(("grp", "all".to_string(), ""))
            .chain(GROUPS.iter().map(|(g, n)| ("grp", n.to_string(), *g)))
            .collect(),
    );
    let lev = chip_row(
        cx,
        "level",
        vec![
            ("level", "all".into(), ""),
            ("level", "error".into(), "error"),
            ("level", "warn".into(), "warn"),
        ],
    );
    let below = chip_row(
        cx,
        "levels",
        vec![
            ("below", "all".into(), ""),
            ("below", "this level and below".into(), "1"),
        ],
    );
    let status = chip_row(
        cx,
        "status",
        vec![
            ("status", "any".into(), ""),
            ("status", "open".into(), "open"),
            ("status", "accepted".into(), "accepted"),
        ],
    );
    let code_chips = code_chips(cx);
    view! {
        (grp)
        (lev)
        (below)
        (status)
        div(class="chips") { span(class="label") { "code" } (code_chips) }
    }
}

/// A labelled row of chips.
fn chip_row(cx: &Cx, label: &'static str, opts: Vec<Chip>) -> View {
    let chips: Vec<View> = opts
        .into_iter()
        .map(|(field, text, value)| {
            nav_link(
                cx,
                text,
                "chip",
                move |p| chip_value(p, field) == value,
                move |p| p.with(|p| *chip_slot(p, field) = value.to_string()),
            )
        })
        .collect();
    let chips = View::from(chips);
    view! { div(class="chips") { span(class="label") { (label) } (chips) } }
}

/// The value of the param a chip `field` sets.
fn chip_value<'a>(p: &'a Params, field: &str) -> &'a str {
    match field {
        "grp" => &p.grp,
        "level" => &p.level,
        "below" => &p.below,
        _ => &p.status,
    }
}

/// The param a chip `field` sets.
fn chip_slot<'a>(p: &'a mut Params, field: &str) -> &'a mut String {
    match field {
        "grp" => &mut p.grp,
        "level" => &mut p.level,
        "below" => &mut p.below,
        _ => &mut p.status,
    }
}

/// An `all` chip and one chip per finding code present.
fn code_chips(cx: &Cx) -> View {
    let codes: BTreeSet<&String> = cx.m.diags.iter().map(|d| &d.code).collect();
    let mut chips: Vec<View> = vec![nav_link(
        cx,
        "all".into(),
        "chip",
        |p| p.code.is_empty(),
        |p| p.with(|p| p.code.clear()),
    )];
    for c in codes {
        let (a, b, label) = (c.clone(), c.clone(), c.clone());
        chips.push(nav_link(
            cx,
            label,
            "chip",
            move |p| p.code == a,
            move |p| p.with(|p| p.code = b.clone()),
        ));
    }
    View::from(chips)
}

/// The findings list: title, then a row per finding or a hint when there is none.
fn findings_list(cx: &Cx) -> impl Fn() -> View + 'static {
    let cx = cx.clone();
    move || {
        let p = cx.params.get_clone();
        let idx = current_list(&cx);
        let (n, total) = (idx.len(), cx.m.diags.len());
        let title = list_title(n, total);
        let tip = format!(
            "{n} shown of {total}: the data layers, the selected scope, the chips and the text filter all narrow the list"
        );
        let rows: Vec<View> = idx.iter().map(|&i| finding_row(&cx, i)).collect();
        let rows = if rows.is_empty() {
            let hint = empty_hint(p.preset());
            view! { p(class="muted pad") { (hint) } }
        } else {
            View::from(rows)
        };
        view! { h3(title=tip) { (title) } (rows) }
    }
}

/// `Findings (n)`, or `Findings (n of total)` while a filter narrows the list.
fn list_title(n: usize, total: usize) -> String {
    if n != total {
        format!("Findings ({n} of {total})")
    } else {
        format!("Findings ({n})")
    }
}

/// What an empty list says in the given preset.
fn empty_hint(preset: &str) -> &'static str {
    if preset == "arch" {
        "Viewing Architecture: press Violations (key 2) to list findings."
    } else {
        "No findings in this scope."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_reads_and_writes_the_same_param() {
        for field in ["grp", "level", "below", "status"] {
            let mut p = Params::default();
            *chip_slot(&mut p, field) = "x".to_string();
            assert_eq!(chip_value(&p, field), "x");
        }
        let mut p = Params::default();
        *chip_slot(&mut p, "level") = "warn".to_string();
        assert_eq!((p.level.as_str(), p.grp.as_str()), ("warn", ""));
        // An unknown field falls back to the status param.
        *chip_slot(&mut p, "other") = "open".to_string();
        assert_eq!(p.status, "open");
    }

    #[test]
    fn title_shows_the_total_only_when_narrowed() {
        assert_eq!(list_title(3, 3), "Findings (3)");
        assert_eq!(list_title(2, 5), "Findings (2 of 5)");
    }

    #[test]
    fn hint_depends_on_the_preset() {
        assert!(empty_hint("arch").starts_with("Viewing Architecture"));
        assert_eq!(empty_hint("viol"), "No findings in this scope.");
    }
}
