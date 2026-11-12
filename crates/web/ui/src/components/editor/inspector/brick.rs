//! The brick inspector: state and file, the agent review bar, auto-applying fields (name,
//! description, tags, members, path), Make group / Ungroup / Delete, and what the rules decide
//! now for its links.

use super::super::op;
use super::super::widgets::{chips_field, text_field};
use super::review_bar;
use crate::components::call::call;
use crate::state::*;
use brickie_web_api::change::Op;
use brickie_web_api::{BrickView, ChangeLinks, ChangeLinksRequest};
use sycamore::prelude::*;

/// Fetch the link options of the brick at `path` into `ed.links`.
fn fetch_links(ed: Edit, path: Vec<String>) {
    ed.links.set(Vec::new());
    call::<ChangeLinks>(&ChangeLinksRequest { path }, move |r| match r {
        Ok(v) => ed.links.set(v),
        Err(e) => ed.msg.set(e),
    });
}

fn set(cx: &Cx, file: &str, b: &BrickView, f: Field) {
    if let Some(o) = set_field(file, b, f) {
        op(cx, o);
    }
}

fn fields(cx: &Cx, file: String, b: &BrickView) -> View {
    let (c, f, b0) = (cx.clone(), file.clone(), b.clone());
    let name = text_field("brick-name", "name", b.name.clone(), false, move |to| {
        super::super::rename(&c, &f, &b0.name, &to)
    });
    let (c, f, b0) = (cx.clone(), file.clone(), b.clone());
    let desc = text_field(
        "brick-desc",
        "description",
        b.desc.clone().unwrap_or_default(),
        true,
        move |d| set(&c, &f, &b0, Field::Desc(d)),
    );
    let (c, f, b0) = (cx.clone(), file.clone(), b.clone());
    let tags = chips_field("brick-tag", "tags", b.tags.clone(), "+ tag", move |t| {
        set(&c, &f, &b0, Field::Tags(t))
    });
    let (c, f, b0) = (cx.clone(), file.clone(), b.clone());
    let members = chips_field(
        "brick-member",
        "members",
        b.members.clone(),
        "+ crate, crate::mod, module:, path:",
        move |m| set(&c, &f, &b0, Field::Members(m)),
    );
    let path = b.composite().then(|| {
        let (c, f, b0) = (cx.clone(), file.clone(), b.clone());
        text_field(
            "brick-path",
            "path (group dir)",
            String::new(),
            false,
            move |p| set(&c, &f, &b0, Field::Path(p)),
        )
    });
    view! { div(class="form") { (name) (desc) (tags) (members) (path) } }
}

/// A composite, or a group the change just added (its level exists, still empty).
fn is_group(cx: &Cx, b: &BrickView) -> bool {
    let key = layout_key(&cx.level().path, &b.name);
    b.composite()
        || cx
            .m
            .change
            .as_deref()
            .is_some_and(|ch| ch.files.contains_key(&key))
}

fn actions(cx: &Cx, file: String, b: &BrickView) -> View {
    let name = b.name.clone();
    let (c, f, n) = (cx.clone(), file.clone(), name.clone());
    let group = move |_: Click| {
        op(
            &c,
            Op::MakeComposite {
                file: f.clone(),
                name: n.clone(),
                path: n.clone(),
            },
        )
    };
    let (c, f, n) = (cx.clone(), file, name);
    let ungroup = move |_: Click| {
        op(
            &c,
            Op::Ungroup {
                file: f.clone(),
                name: n.clone(),
            },
        )
    };
    let toggle = if is_group(cx, b) {
        view! { button(id="ungroup", title="Lift the children to this level; the brick stays a leaf", on:click=ungroup) { "Ungroup" } }
    } else {
        view! { button(id="make-group", title="Give the brick its own level (brickie.toml in a folder of its name)", on:click=group) { "Make group" } }
    };
    let ed = cx.ed;
    view! {
        div(class="row acts") {
            (toggle)
            button(class="danger", on:click=move |_: Click| ed.confirm.set(true)) { "Delete\u{2026}" }
        }
    }
}

fn now_lines(ed: Edit) -> View {
    let rows = move || {
        let rows: Vec<View> = ed.links.with(|l| {
            l.iter()
                .map(|o| {
                    let cls = format!("la {}", o.action);
                    let (to, k, a) = (
                        o.to.clone(),
                        format!("{} {}", o.kind, o.scope),
                        o.action.clone(),
                    );
                    let by = o.rule.clone().unwrap_or_else(|| "deps.default".into());
                    view! { tr(title=by) { td { (to) } td { (k) } td(class=cls) { (a) } } }
                })
                .collect()
        });
        View::from(rows)
    };
    view! { details { summary { "What the rules decide now" } table(class="tr") { (rows) } } }
}

pub fn brick_view(cx: &Cx, b: usize) -> View {
    let brick = cx.m.g.bricks[b].clone();
    let lv = cx.level().path.clone();
    fetch_links(cx.ed, brick_path(&lv, &brick.name));
    let Some(file) = cx.level_file() else {
        return view! { p(class="err-text") { "This level is not in the preview." } };
    };
    let seqs: Vec<u64> =
        cx.m.change
            .as_deref()
            .map(|ch| {
                proposals_on(ch, &lv, &brick.name)
                    .iter()
                    .map(|r| r.seq)
                    .collect()
            })
            .unwrap_or_default();
    let state = match (brick.planned, brick.proposed) {
        (_, true) => "proposed by the agent",
        (true, _) => "planned: no code yet (publish scaffolds it)",
        _ => "built",
    };
    let zoom = is_group(cx, &brick).then(|| {
        let (c, target) = (cx.clone(), layout_key(&lv, &brick.name));
        view! { button(on:click=move |_: Click| c.go(c.params.with(|p| p.at_level(&target)))) { "Open its level" } }
    });
    let sub = format!("{state} \u{00B7} in {file}");
    let title = brick.name.clone();
    let (rv, f, a) = (
        review_bar(cx, &seqs),
        fields(cx, file.clone(), &brick),
        actions(cx, file, &brick),
    );
    let now = now_lines(cx.ed);
    view! {
        (rv)
        h2 { (title) }
        p(class="muted") { (sub) }
        (zoom)
        (f)
        (a)
        p(class="muted") { "Drag a typed port on the brick's right edge onto a sibling to add a rule." }
        (now)
    }
}
