//! The rule or link inspector: kinds, scope and reason (required); its file and who shares
//! it; a link no rule allows says why. Each change is one version, named by identity. Design
//! mode has no measured edge.

use super::super::widgets::{seg, text_field};
use super::super::{keys::delete_selection, ops_then};
use crate::state::*;
use brickie_web_api::design::RuleSpec;
use sycamore::prelude::*;

const KINDS: [&str; 3] = ["uses", "implements", "reexports"];

/// Change rule or link `r` into `rule` and keep it selected (its identity may change).
fn set_rule(cx: &Cx, r: &RuleRef, rule: RuleSpec) {
    let Some(old) = cx.m.design.as_deref().and_then(|d| rule_at(d, r)).cloned() else {
        return;
    };
    let next = RuleRef {
        key: rule_key(&rule),
        ..r.clone()
    };
    ops_then(cx, rule_ops(r, &old, &rule), move |now, _| {
        now.ed.rule.set(Some(next));
    });
}

/// Kind toggles (none on = every kind).
fn kinds(cx: &Cx, r: &RuleRef, rule: &RuleSpec) -> View {
    let items: Vec<View> = KINDS
        .iter()
        .map(|&k| {
            let on = rule.kinds.iter().any(|x| x == k);
            let (c, r, mut next) = (cx.clone(), r.clone(), rule.clone());
            next.kinds = toggled(&rule.kinds, k, !on);
            let cls = if on { format!("kchip l-{k} on") } else { format!("kchip l-{k}") };
            view! { button(class=cls, on:click=move |_: Click| set_rule(&c, &r, next.clone())) { (k) } }
        })
        .collect();
    let items = View::from(items);
    view! { label { "kinds (none: all)" } div(class="row") { (items) } }
}

fn choices(cx: &Cx, r: &RuleRef, rule: &RuleSpec) -> View {
    let (c, rr, base) = (cx.clone(), r.clone(), rule.clone());
    let scope = seg(&SCOPES, rule.scope.as_deref().unwrap_or("all"), move |s| {
        set_rule(
            &c,
            &rr,
            RuleSpec {
                scope: Some(s.to_string()),
                ..base.clone()
            },
        )
    });
    view! { label { "scope" } (scope) }
}

fn texts(cx: &Cx, r: &RuleRef, rule: &RuleSpec) -> View {
    let (c, rr, base) = (cx.clone(), r.clone(), rule.clone());
    let reason = text_field(
        "rule-reason",
        "reason (required)",
        rule.reason.clone(),
        true,
        move |s| {
            let s = s.trim().to_string();
            if s.is_empty() {
                c.ed.msg
                    .set("A reason is required: kept the old one.".into());
                return;
            }
            set_rule(
                &c,
                &rr,
                RuleSpec {
                    reason: s,
                    ..base.clone()
                },
            )
        },
    );
    view! { (reason) }
}

pub fn rule_view(cx: &Cx, r: RuleRef) -> View {
    let Some(ch) = cx.m.design.clone() else {
        return View::default();
    };
    let Some(rule) = rule_at(&ch, &r).cloned() else {
        return view! { p(class="muted") { "It is gone." } };
    };
    let n = sharers(&ch, &r.file);
    let shared = (n > 1).then(|| view! { p(class="warn-text") { (format!("Shared by {n} levels: editing it changes all of them.")) } });
    let served = (cx.level().links.iter())
        .find(|v| r.link && v.file == r.file && v.from == rule.from && v.to == rule.to);
    let denied = served.and_then(|v| v.denied.clone()).map(|why| view! { p(class="err-text") { (format!("Not allowed: {why}. It blocks publishing.")) } });
    let cycle = served
        .is_some_and(|v| v.cycle)
        .then(|| view! { p(class="warn-text") { "On a cycle of this level's links." } });
    let what = if r.link { "link" } else { "rule" };
    let title = format!("{} \u{2192} {}", rule.from, rule.to);
    let sub = format!("{what} in {}", r.file);
    let del = format!("Delete {what}");
    let c = cx.clone();
    let copy = super::copy_ref(rule_ref(ch.sheets.get(&r.file), r.link, &rule));
    let (k, ch2, t) = (
        kinds(cx, &r, &rule),
        choices(cx, &r, &rule),
        texts(cx, &r, &rule),
    );
    view! {
        h2 { (title) (copy) }
        p(class="muted mono") { (sub) }
        (shared)
        (denied)
        (cycle)
        div(class="form") { (t) (k) (ch2) }
        div(class="row acts") { button(class="danger", on:click=move |_: Click| delete_selection(&c)) { (del) } }
    }
}
