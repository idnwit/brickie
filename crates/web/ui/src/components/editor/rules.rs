//! The Rules tab: every `[[rule]]` that applies at the level (its own file and includes) and
//! every `[[link]]` declared there, each with its physical file; a row opens it in the
//! inspector, `x` removes it. The add row takes selectors of the level and a required reason.
//! A link no rule allows is marked with why.

use super::op;
use crate::state::*;
use brickie_web_api::design::{Op, RuleSpec};
use sycamore::prelude::*;

/// Why link `r` is not allowed, as served for the level on screen.
fn denied(cx: &Cx, r: &Row<RuleSpec>) -> Option<String> {
    (cx.level().links.iter())
        .find(|v| v.file == r.file && v.from == r.item.from && v.to == r.item.to)
        .and_then(|v| v.denied.clone())
}

fn row(cx: &Cx, own: &str, r: Row<RuleSpec>, link: bool, why: Option<String>) -> View {
    let ed = cx.ed;
    let rr = if link { r.link_ref() } else { r.rule_ref() };
    let on = ed.rule.with(|x| x.as_ref() == Some(&rr));
    let cls = format!(
        "rrow{}{}{}",
        if r.file != own { " inc" } else { "" },
        if on { " on" } else { "" },
        if why.is_some() { " denied" } else { "" }
    );
    let kinds = if r.item.kinds.is_empty() {
        "all".to_string()
    } else {
        r.item.kinds.join("+")
    };
    let (c, file, key) = (cx.clone(), r.file.clone(), rr.key.clone());
    let rm = move |e: Click| {
        e.stop_propagation();
        let (file, key) = (file.clone(), key.clone());
        op(
            &c,
            if link {
                Op::RemoveLink { file, key }
            } else {
                Op::RemoveRule { file, key }
            },
        );
    };
    let pick = move |_: Click| {
        ed.rule.set(Some(rr.clone()));
        ed.tab.set("inspect");
    };
    let at = match &why {
        Some(w) => format!("{}: not allowed: {w}", r.file),
        None => r.file.clone(),
    };
    let (from, to, scope, reason) = (
        r.item.from,
        r.item.to,
        r.item.scope.unwrap_or_else(|| "all".into()),
        r.item.reason,
    );
    view! {
        tr(class=cls, title=at, on:click=pick) {
            td { (from) } td { (to) } td { (kinds) } td { (scope) }
            td(class="muted") { (reason) } td { button(class="px", title="remove", on:click=rm) { "\u{00D7}" } }
        }
    }
}

fn add_row(cx: &Cx, file: String, selectors: Vec<String>) -> View {
    let (from, to) = (create_signal(String::new()), create_signal(String::new()));
    let (ty, reason) = (
        create_signal("uses".to_string()),
        create_signal(String::new()),
    );
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
        let (f, g, why) = (from.get_clone(), to.get_clone(), reason.get_clone());
        if f.is_empty() || g.is_empty() {
            return;
        }
        if why.trim().is_empty() {
            c.ed.msg.set("A rule needs a reason.".into());
            return;
        }
        let rule = RuleSpec {
            from: f,
            to: g,
            kinds: t.kinds(),
            scope: Some(c.ed.scope.get().to_string()),
            reason: why.trim().to_string(),
        };
        op(
            &c,
            Op::AddRule {
                file: file.clone(),
                rule,
            },
        );
        reason.set(String::new());
    };
    view! {
        div(class="row addrule") {
            select(bind:value=from) { option(value="") { "from" } (o1) }
            select(bind:value=ty) { (types) }
            select(bind:value=to) { option(value="") { "to" } (o2) }
            input(bind:value=reason, placeholder="reason (required)")
            button(on:click=add) { "+ rule" }
        }
    }
}

fn table(rows: Vec<View>) -> View {
    let rows = View::from(rows);
    view! {
        table(class="rules") {
            tr { th { "from" } th { "to" } th { "kinds" } th { "scope" } th { "reason" } th {} }
            (rows)
        }
    }
}

pub fn rules(cx: &Cx) -> View {
    let (Some(ch), Some(file)) = (cx.m.design.clone(), cx.level_file()) else {
        return View::default();
    };
    let lv = cx.level().path.clone();
    let rules: Vec<View> = level_rules(&ch, &lv)
        .into_iter()
        .map(|r| row(cx, &file, r, false, None))
        .collect();
    let links: Vec<View> = level_links(&ch, &lv)
        .into_iter()
        .map(|r| {
            let why = denied(cx, &r);
            row(cx, &file, r, true, why)
        })
        .collect();
    let no_rules = rules
        .is_empty()
        .then(|| view! { p(class="muted") { "No rules: no link of this level is allowed." } });
    let no_links = links.is_empty().then(|| view! { p(class="muted") { "No links. Drag a port between bricks on the canvas to add one." } });
    let selectors = ch
        .levels
        .get(&lv)
        .map(|l| l.selectors.clone())
        .unwrap_or_default();
    let a = add_row(cx, file.clone(), selectors);
    let (rt, lt) = (table(rules), table(links));
    view! {
        h3 { "Rules (what links may exist)" }
        p(class="muted mono") { (file) }
        (rt)
        (no_rules)
        (a)
        p(class="muted") { "Tinted rows come from an included file (shared)." }
        h3 { "Links (declared dependencies)" }
        (lt)
        (no_links)
    }
}
