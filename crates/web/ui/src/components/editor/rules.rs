//! The Rules tab: `[deps]` default / cycles of the level's file, and every dependency rule that
//! applies at the level (its own file and includes), each with its physical file; a row opens
//! the rule in the inspector, `x` removes it, and the add row takes selectors of the level.

use super::op;
use super::widgets::seg;
use crate::state::*;
use brickie_web_api::change::{FileSheet, Op};
use sycamore::prelude::*;

fn deps(cx: &Cx, file: &str, sheet: &FileSheet) -> View {
    let set = |default: bool| {
        let (c, f) = (cx.clone(), file.to_string());
        move |v: &'static str| {
            let (d, y) = if default {
                (Some(v.to_string()), None)
            } else {
                (None, Some(v.to_string()))
            };
            op(
                &c,
                Op::SetDeps {
                    file: f.clone(),
                    default: d,
                    cycles: y,
                },
            );
        }
    };
    let d = seg(
        &["allow", "deny"],
        sheet.default.as_deref().unwrap_or("allow"),
        set(true),
    );
    let y = seg(
        &["allow", "deny"],
        sheet.cycles.as_deref().unwrap_or("allow"),
        set(false),
    );
    view! {
        div(class="form") {
            label { "deps.default (edges no rule matches)" } (d)
            label { "deps.cycles" } (y)
        }
    }
}

fn row(cx: &Cx, own: &str, r: Row<brickie_web_api::change::RuleSpec>) -> View {
    let ed = cx.ed;
    let rr = RuleRef {
        file: r.file.clone(),
        index: r.index,
    };
    let on = ed.rule.with(|x| x.as_ref() == Some(&rr));
    let cls = format!(
        "rrow{}{}",
        if r.file != own { " inc" } else { "" },
        if on { " on" } else { "" }
    );
    let kinds = if r.item.kinds.is_empty() {
        "all".to_string()
    } else {
        r.item.kinds.join("+")
    };
    let reason = r.item.reason.clone().unwrap_or_else(|| "?".into());
    let (c, file, index) = (cx.clone(), r.file.clone(), r.index);
    let rm = move |e: Click| {
        e.stop_propagation();
        op(
            &c,
            Op::RemoveRule {
                file: file.clone(),
                index,
            },
        );
    };
    let pick = move |_: Click| {
        ed.rule.set(Some(rr.clone()));
        ed.tab.set("inspect");
    };
    let at = format!("{}#{}", r.file, r.index);
    let (from, to, scope, action) = (
        r.item.from,
        r.item.to,
        r.item.scope.unwrap_or_else(|| "prod".into()),
        r.item.action,
    );
    let acls = format!("la {action}");
    view! {
        tr(class=cls, title=at, on:click=pick) {
            td { (from) } td { (to) } td { (kinds) } td { (scope) } td(class=acls) { (action) }
            td(class="muted") { (reason) } td { button(class="px", title="remove", on:click=rm) { "\u{00D7}" } }
        }
    }
}

fn add_row(cx: &Cx, file: String, selectors: Vec<String>) -> View {
    let (from, to) = (create_signal(String::new()), create_signal(String::new()));
    let ty = create_signal("uses".to_string());
    let opts = |list: &[String]| {
        View::from(
            list.iter()
                .map(|s| {
                    let (v, t) = (s.clone(), s.clone());
                    view! { option(value=v) { (t) } }
                })
                .collect::<Vec<_>>(),
        )
    };
    let types = View::from(
        LINK_TYPES
            .iter()
            .map(|t| view! { option(value=t.name()) { (t.name()) } })
            .collect::<Vec<_>>(),
    );
    let (o1, o2) = (opts(&selectors), opts(&selectors));
    let c = cx.clone();
    let add = move |_: Click| {
        let t = LINK_TYPES
            .into_iter()
            .find(|t| ty.with(|x| x == t.name()))
            .unwrap_or(LinkType::Uses);
        let (f, g) = (from.get_clone(), to.get_clone());
        if !f.is_empty() && !g.is_empty() {
            op(&c, link_op(&file, &f, &g, t, c.ed.scope.get()));
        }
    };
    view! {
        div(class="row addrule") {
            select(bind:value=from) { option(value="") { "from" } (o1) }
            select(bind:value=ty) { (types) }
            select(bind:value=to) { option(value="") { "to" } (o2) }
            button(on:click=add) { "+ rule" }
        }
    }
}

pub fn rules(cx: &Cx) -> View {
    let (Some(ch), Some(file)) = (cx.m.change.clone(), cx.level_file()) else {
        return View::default();
    };
    let lv = cx.level().path.clone();
    let sheet = ch.sheets.get(&file).cloned().unwrap_or_default();
    let rows: Vec<View> = level_rules(&ch, &lv)
        .into_iter()
        .map(|r| row(cx, &file, r))
        .collect();
    let empty = rows.is_empty().then(|| view! { p(class="muted") { "No rules. Drag a port between bricks on the canvas to add one." } });
    let selectors = ch
        .levels
        .get(&lv)
        .map(|l| l.selectors.clone())
        .unwrap_or_default();
    let (d, a) = (
        deps(cx, &file, &sheet),
        add_row(cx, file.clone(), selectors),
    );
    let rows = View::from(rows);
    view! {
        h3 { "Dependency rules" }
        p(class="muted mono") { (file) }
        (d)
        table(class="rules") {
            tr { th { "from" } th { "to" } th { "kinds" } th { "scope" } th { "action" } th { "reason" } th {} }
            (rows)
        }
        (empty)
        (a)
        p(class="muted") { "Tinted rows come from an included file (shared)." }
    }
}
