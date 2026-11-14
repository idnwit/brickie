//! Editor keys and the actions they share with buttons: `E` design mode, `V`/`H` tools, `B`
//! brick tool (`Tab` cycles the kind), `L` link tool, `1`/`2`/`3` scope, `F2` rename, `Del`
//! delete, `Ctrl+Z` undo, `Ctrl+Shift+Z` / `Ctrl+Y` redo, `Ctrl+X`/`Ctrl+V` cut and paste a
//! brick across levels, `?` shortcuts, `+`/`-`/`0` zoom.

use super::{op, write};
use crate::platform;
use crate::state::*;
use brickie_web_api::design::{BrickKind, Op};
use brickie_web_api::*;

/// Design (edit) mode on/off (a page load: edit mode reads the design as a draft).
pub fn toggle_design(cx: &Cx) {
    let on = cx.params.with(|p| p.editing());
    let p = cx
        .params
        .with(|p| p.with(|p| p.edit = if on { String::new() } else { "1".into() }));
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
                key: r.key,
            },
        );
        return;
    }
    match cx.eval.with(|ev| ev.sel.clone()) {
        Sel::Brick(b) if !cx.m.g.bricks[b].ghost && !cx.m.g.bricks[b].unassigned => {
            ed.confirm.set(true)
        }
        Sel::Edge(_) => ed
            .msg
            .set("A measured edge is code: change the code to remove it.".into()),
        _ => {}
    }
}

/// Revert version `v` (`None`: nothing to revert), toast `word: what it did`.
fn revert(cx: &Cx, v: Option<u64>, word: &'static str) {
    let Some(v) = v else {
        cx.ed
            .msg
            .set(format!("Nothing to {}.", word.to_lowercase()));
        return;
    };
    let tip =
        cx.m.design
            .as_deref()
            .map(|d| undo_tip(word, &d.history, Some(v)));
    let req = UndoRequest {
        base: 0,
        v: Some(v),
    };
    write::<DesignUndo>(cx, req, tip.unwrap_or_default(), |_, _| {});
}

/// Undo the human's latest edit (`Ctrl+Z`): a new version reverting it.
pub fn undo(cx: &Cx) {
    let v = cx.m.design.as_deref().and_then(|d| undo_target(&d.history));
    revert(cx, v, "Undo");
}

/// Redo (`Ctrl+Shift+Z` / `Ctrl+Y`): revert the latest undo.
pub fn redo(cx: &Cx) {
    let v = cx.m.design.as_deref().and_then(|d| redo_target(&d.history));
    revert(cx, v, "Redo");
}

/// Revert version `v` made by another writer (the toast's Undo).
pub fn undo_version(cx: &Cx, v: u64) {
    revert(cx, Some(v), "Undo");
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
            .for_each(|s| cx.view.set(cx.fit(s))),
        k => match scope_key(k) {
            Some(s) => ed.scope.set(s),
            None => return false,
        },
    }
    true
}

fn rename_selected(cx: &Cx) {
    if let Sel::Brick(b) = cx.eval.with(|ev| ev.sel.clone()) {
        if cx.m.g.bricks[b].unassigned {
            return;
        }
        cx.ed.rename.set(Some(cx.m.g.bricks[b].name.clone()));
        platform::next_frame(|| platform::focus("inline-name"));
    }
}

fn edits(cx: &Cx, key: &str) -> bool {
    match key {
        "Ctrl+z" => undo(cx),
        "Ctrl+x" => super::clip::cut(cx),
        "Ctrl+v" => super::clip::paste(cx),
        "?" => cx.ed.help.set(!cx.ed.help.get()),
        "Ctrl+Shift+z" | "Ctrl+y" => redo(cx),
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
        return None;
    }
    (tools(cx, key) || edits(cx, key)).then_some(true)
}
