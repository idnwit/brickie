//! Editor keys and the actions they share with buttons: `E` design mode, `V`/`H` tools, `B`
//! brick tool (`Tab` cycles the kind), `L` link tool, `1`/`2`/`3` scope, `F2` rename, `Del`
//! delete, `Ctrl+Z` retract, `A`/`R`/`N` review proposals, `P`/`C`/`D` preview / current /
//! diff, `+`/`-`/`0` zoom.

use super::{op, write};
use crate::platform;
use crate::state::*;
use brickie_web_api::change::{BrickKind, Op, Verdict};
use brickie_web_api::*;

/// Design on/off: opens a change first when none is open.
pub fn toggle_design(cx: &Cx) {
    if cx.m.change.is_none() {
        cx.ed.dialog.set(Dialog::Open);
        return;
    }
    let on = cx.editing();
    let p = cx
        .params
        .with(|p| p.with(|p| p.change = if on { String::new() } else { "1".into() }));
    platform::goto(&p.url());
}

/// Delete what is selected: a rule, or a brick (after the confirmation popover).
pub fn delete_selection(cx: &Cx) {
    let ed = cx.ed;
    if let Some(r) = ed.rule.get_clone() {
        ed.rule.set(None);
        op(
            cx,
            Op::RemoveRule {
                file: r.file,
                index: r.index,
            },
        );
        return;
    }
    match cx.eval.with(|ev| ev.sel.clone()) {
        Sel::Brick(b) if !cx.m.g.bricks[b].ghost => ed.confirm.set(true),
        Sel::Edge(_) => ed
            .msg
            .set("A measured edge is code: change the code to remove it.".into()),
        _ => {}
    }
}

/// Take back the latest human op (`Ctrl+Z`).
pub fn retract(cx: &Cx) {
    let req = ChangeRetractRequest { seq: None };
    write::<ChangeRetract, _>(cx, req, "retracted the last op".into(), |_, _| {});
}

/// Accept or reject op `seq`.
pub fn review(cx: &Cx, seq: u64, verdict: Verdict, note: Option<String>) {
    let toast = format!("{verdict:?} #{seq}").to_lowercase();
    let req = ChangeReviewRequest { seq, verdict, note };
    write::<ChangeReview, _>(cx, req, toast, |_, _| {});
}

/// The pending agent op of the selection (a brick or a planned rule).
pub fn selected_proposal(cx: &Cx) -> Option<u64> {
    let ch = cx.m.change.as_deref()?;
    if let Some(r) = cx.ed.rule.get_clone() {
        let e = ch
            .planned_edges
            .iter()
            .find(|e| e.file == r.file && e.rule == r.index)?;
        return e
            .seq
            .filter(|s| ch.log.iter().any(|o| o.seq == *s && pending(o)));
    }
    let Sel::Brick(b) = cx.eval.with(|ev| ev.sel.clone()) else {
        return None;
    };
    let name = &cx.m.g.bricks[b].name;
    proposals_on(ch, &cx.level().path, name)
        .first()
        .map(|r| r.seq)
}

/// Jump to the next pending agent op: its level, its brick or planned rule.
pub fn next(cx: &Cx) {
    let Some(ch) = cx.m.change.as_deref() else {
        return;
    };
    let after = selected_proposal(cx).unwrap_or(0);
    let Some(r) = next_proposal(ch, after) else {
        cx.ed.msg.set("No proposals to review.".into());
        return;
    };
    let level = op_level(ch, r).unwrap_or_else(|| "root".into());
    let edge = ch.planned_edges.iter().find(|e| e.seq == Some(r.seq));
    cx.ed.rule.set(edge.map(|e| RuleRef {
        file: e.file.clone(),
        index: e.rule,
    }));
    let brick = op_brick(&r.op).unwrap_or_default().to_string();
    let p = cx.params.with(|p| p.at_level(&level)).with(|p| {
        p.brick = if edge.is_some() {
            String::new()
        } else {
            brick.clone()
        };
    });
    let here = level == cx.level().path;
    cx.go(p);
    cx.ed.tab.set("inspect");
    if let Some(b) = cx.brick(&brick).filter(|_| here) {
        center_on(cx, b);
    }
}

/// Pan so brick `b` is in the middle of the stage (zoom kept).
fn center_on(cx: &Cx, b: usize) {
    let Some((sw, sh)) = cx.stage.get() else {
        return;
    };
    let r = positions(&cx.m.layout, &cx.drag.get_clone())[b];
    let k = cx.view.get().k;
    cx.view.set(Vw {
        x: sw / 2.0 - (r.x + r.w / 2.0) * k,
        y: sh / 2.0 - (r.y + r.h / 2.0) * k,
        k,
    });
}

fn zoom(cx: &Cx, f: f64) {
    let Some((sw, sh)) = cx.stage.get() else {
        return;
    };
    let v = cx.view.get();
    let k = (v.k * f).clamp(0.15, 3.0);
    let (mx, my) = (sw / 2.0, sh / 2.0);
    cx.view.set(Vw {
        x: mx - (mx - v.x) * (k / v.k),
        y: my - (my - v.y) * (k / v.k),
        k,
    });
}

fn show(cx: &Cx, preview: bool, diff: bool) {
    cx.ed.diff.set(diff);
    if preview != cx.editing() {
        toggle_design(cx);
    }
}

fn brick_tool(ed: Edit, tab: bool) {
    let next = match (ed.brick_tool.get(), tab) {
        (Some(BrickKind::Crate), true) => BrickKind::Module,
        (Some(BrickKind::Module), true) => BrickKind::Group,
        (Some(k), false) => k,
        _ => BrickKind::Crate,
    };
    ed.link_type.set(None);
    ed.brick_tool.set(Some(next));
}

fn tools(cx: &Cx, key: &str) -> bool {
    let ed = cx.ed;
    match key {
        "v" => ed.tool.set(Tool::Select),
        "h" => ed.tool.set(Tool::Hand),
        "b" => brick_tool(ed, false),
        "Tab" if ed.brick_tool.get().is_some() => brick_tool(ed, true),
        "l" => ed
            .link_type
            .set(Some(ed.link_type.get().unwrap_or(LinkType::Uses))),
        "F2" => rename_selected(cx),
        "Delete" => delete_selection(cx),
        "+" | "=" => zoom(cx, 1.2),
        "-" => zoom(cx, 1.0 / 1.2),
        "0" => cx
            .stage
            .get()
            .into_iter()
            .for_each(|s| cx.view.set(fit_view(&cx.m.layout, s))),
        k => match scope_key(k) {
            Some(s) => ed.scope.set(s),
            None => return false,
        },
    }
    true
}

fn rename_selected(cx: &Cx) {
    if let Sel::Brick(b) = cx.eval.with(|ev| ev.sel.clone()) {
        cx.ed.rename.set(Some(cx.m.g.bricks[b].name.clone()));
        platform::next_frame(|| platform::focus("inline-name"));
    }
}

fn reviews(cx: &Cx, key: &str) -> bool {
    match key {
        "a" => match selected_proposal(cx) {
            Some(seq) => review(cx, seq, Verdict::Accept, None),
            None => return false,
        },
        "r" => platform::focus("review-note"),
        "n" => next(cx),
        "Ctrl+z" => retract(cx),
        "p" => show(cx, true, false),
        "c" => show(cx, false, false),
        "d" => show(cx, true, true),
        _ => return false,
    }
    true
}

/// An editor key: `Some(prevent default)` when the editor handled it.
pub fn key(cx: &Cx, key: &str) -> Option<bool> {
    if key == "e" || key == "E" {
        toggle_design(cx);
        return Some(true);
    }
    if !cx.editing() {
        let open = cx.m.change.is_some() && matches!(key, "p" | "d");
        return open.then(|| show(cx, true, key == "d")).map(|_| true);
    }
    (tools(cx, key) || reviews(cx, key)).then_some(true)
}
