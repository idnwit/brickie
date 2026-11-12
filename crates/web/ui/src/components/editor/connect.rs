//! Connecting, type first: dragging a typed port of a brick draws a rubber band; the sibling or
//! tag lane under the pointer is checked (`/api/change/link-check`) and a redundant target is
//! refused ("already allowed"). Dropping adds the rule at once (`add_rule`), selects it and
//! focuses its reason in the inspector.

use super::{op_then, send};
use crate::components::call::call;
use crate::components::canvas::frame_rect;
use crate::platform;
use crate::state::*;
use brickie_web_api::{ChangeLinkCheck, ChangeLinkCheckRequest};

/// Start dragging port `ty` of brick `from` at world point `(x, y)`.
pub fn start(ed: Edit, from: String, ty: LinkType, x: f64, y: f64) {
    ed.check.set(None);
    ed.connect.set(Some(Connect { from, ty, x, y }));
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
        cx.m.change
            .as_deref()
            .and_then(|c| c.levels.get(&cx.level().path))
            .map(|l| l.tags.clone())?;
    inside
        .then(|| lane_at(&cx.m.layout.rows, &tags, at.1))
        .flatten()
        .map(|t| format!("tag:{t}"))
}

fn check_req(cx: &Cx, c: &Connect, to: &str) -> ChangeLinkCheckRequest {
    ChangeLinkCheckRequest {
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
    let Some(to) = to else { return };
    let req = check_req(cx, &c, &to);
    call::<ChangeLinkCheck>(&req, move |r| {
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
    let Some(file) = cx.level_file() else { return };
    let req = check_req(cx, &c, &to);
    let cx2 = cx.clone();
    send::<ChangeLinkCheck>(ed, req, move |chk| {
        if chk.redundant {
            ed.msg.set(format!("Already allowed: {}", chk.why));
            return;
        }
        let op = link_op(&file, &c.from, &to, c.ty, cx2.ed.scope.get());
        op_then(&cx2, op, move |now, _| select_new_rule(now, &file));
    });
}

/// Select the rule just added to `file` and focus its reason.
fn select_new_rule(cx: &Cx, file: &str) {
    let Some(r) = cx.m.change.as_deref().and_then(|ch| last_rule(ch, file)) else {
        return;
    };
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
