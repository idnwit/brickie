//! Connecting, type first: dragging a typed port of a brick draws a rubber band; the sibling or
//! tag lane under the pointer is checked (`/api/design/link-check`) and a redundant target is
//! refused ("already allowed"). Dropping adds the rule at once (`add_rule`), selects it and
//! focuses its reason in the inspector. Dragging the arrowhead of the selected rule
//! (`retarget`) draws the same band and changes that rule's target instead.

use super::{ops_then, send};
use crate::components::call::call;
use crate::components::canvas::frame_rect;
use crate::platform;
use crate::state::*;
use brickie_web_api::design::{Op, RuleSpec};
use brickie_web_api::{DesignLinkCheck, LinkCheckRequest};

/// Start dragging port `ty` of brick `from` at world point `(x, y)`.
pub fn start(ed: Edit, from: String, ty: LinkType, x: f64, y: f64) {
    ed.check.set(None);
    ed.connect.set(Some(Connect {
        from,
        ty,
        retarget: None,
        x,
        y,
    }));
}

/// The selector under world point `at` for a link from `from`: a sibling or a tag lane.
pub fn target_at(cx: &Cx, from: &str, at: (f64, f64)) -> Option<String> {
    let pos = positions(&cx.m.layout, &cx.drag.get_clone_untracked());
    let skip = cx.brick(from);
    if let Some(t) = hit(&pos, &cx.m.g.bricks, at, skip) {
        return Some(cx.m.g.bricks[t].name.clone());
    }
    if hit_any(&pos, at, None).is_some() {
        return None;
    }
    let inside = frame_rect(&cx.m.g, &pos).is_some_and(|f| at.0 >= f.x - 60.0 && at.0 <= f.x + f.w);
    let tags =
        cx.m.design
            .as_deref()
            .and_then(|c| c.levels.get(&cx.level().path))
            .map(|l| l.tags.clone())?;
    inside
        .then(|| lane_at(&cx.m.layout.rows, &tags, at.1))
        .flatten()
        .map(|t| format!("tag:{t}"))
}

fn check_req(cx: &Cx, c: &Connect, to: &str) -> LinkCheckRequest {
    LinkCheckRequest {
        level: level_brick_path(&cx.level().path),
        from: c.from.clone(),
        to: to.to_string(),
        kinds: c.ty.kinds(),
        scope: cx.ed.scope.get().to_string(),
        action: c.ty.action().to_string(),
    }
}

/// The pointer moved to world point `at` while connecting.
pub fn moved(cx: &Cx, at: (f64, f64)) {
    let ed = cx.ed;
    let Some(c) = ed.connect.get_clone() else {
        return;
    };
    ed.connect.set(Some(Connect {
        x: at.0,
        y: at.1,
        ..c.clone()
    }));
    let to = target_at(cx, &c.from, at);
    let same = ed.check.with(|k| k.as_ref().map(|(t, _)| t.clone())) == to;
    if same {
        return;
    }
    ed.check.set(None);
    let Some(to) = to.filter(|_| c.retarget.is_none()) else {
        return;
    };
    let req = check_req(cx, &c, &to);
    call::<DesignLinkCheck>(&req, move |r| {
        let still = ed.connect.with(|k| k.is_some());
        if let (Ok(v), true) = (r, still) {
            ed.check.set(Some((to, v)));
        }
    });
}

/// Release while connecting: add the rule to the target, unless it changes nothing.
pub fn finish(cx: &Cx, at: Option<(f64, f64)>) {
    let ed = cx.ed;
    let Some(c) = ed.connect.get_clone() else {
        return;
    };
    ed.connect.set(None);
    let Some(to) = at.and_then(|p| target_at(cx, &c.from, p)) else {
        return;
    };
    if let Some(r) = &c.retarget {
        return retarget(cx, r, &to);
    }
    let Some(file) = cx.level_file() else { return };
    let req = check_req(cx, &c, &to);
    let cx2 = cx.clone();
    send::<DesignLinkCheck>(ed, req, move |chk| {
        if chk.redundant {
            let link = format!("{} \u{2192} {to}", c.from);
            ed.msg.set(format!("No rule added for {link}: {}", chk.why));
            return;
        }
        let rule = link_rule(&c.from, &to, c.ty, cx2.ed.scope.get());
        let r = RuleRef::of(&file, &rule);
        let op = Op::AddRule { file, rule };
        ops_then(&cx2, vec![op], move |now, _| select_rule(now, r));
    });
}

/// Point rule `r` at `to` (its arrowhead was dropped there), keeping it selected.
fn retarget(cx: &Cx, r: &RuleRef, to: &str) {
    let rule =
        cx.m.design
            .as_deref()
            .and_then(|ch| rule_at(ch, r))
            .cloned();
    let Some(rule) = rule else { return };
    let Some(ops) = retarget_ops(r, &rule, to) else {
        return;
    };
    let next = RuleRef::of(
        &r.file,
        &RuleSpec {
            to: to.to_string(),
            ..rule
        },
    );
    ops_then(cx, ops, move |now, _| now.ed.rule.set(Some(next)));
}

/// Select rule `r` (just added) and focus its reason.
fn select_rule(cx: &Cx, r: RuleRef) {
    let p = cx.params.with(|p| {
        p.with(|p| {
            p.brick.clear();
            p.edge.clear();
            p.group.clear();
        })
    });
    cx.go(p);
    cx.ed.rule.set(Some(r));
    cx.ed.tab.set("inspect");
    platform::next_frame(|| platform::focus("rule-reason"));
}
