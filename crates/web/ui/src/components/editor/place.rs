//! Committing a drop: the inline name box of a new brick (here, in a lane, in a new lane or
//! inside a brick), the tag box of a brick dropped on the new-lane zone, and applying the ops
//! of one gesture as one version (one `Ctrl+Z`).

use super::overlay::{at, stop};
use super::toolbar::kind_name;
use super::{ops_then, select_brick, send};
use crate::state::*;
use brickie_web_api::LayoutHint;
use brickie_web_api::design::{BrickKind, Op};
use brickie_web_api::*;
use sycamore::prelude::*;

/// Apply the ops of one gesture as one version (one `Ctrl+Z`), then `then`.
pub fn record(cx: &Cx, ops: Vec<Op>, then: impl FnOnce(&Cx) + 'static) {
    ops_then(cx, ops, move |now, _| then(now));
}

/// Add the drafted brick `name`: where it goes decides its file, tags and ops.
fn commit(cx: &Cx, d: Draft, name: String, members: String, tag: String) {
    let file = cx.level_file().unwrap_or_default();
    let lv = cx.level().path.clone();
    let (tags, into) = match &d.dest {
        Dest::Here(t) => (t.clone(), None),
        Dest::NewLane => (split_list(&tag), None),
        Dest::Into(p) => (vec![], cx.brick(p)),
    };
    let spec = tile_spec(d.kind, &name, &members, tags);
    if let Some(p) = into {
        let parent = cx.m.g.bricks[p].clone();
        let ops = add_inside_ops(&file, &parent, spec);
        let note = format!("Added {name} into {} \u{00B7} Ctrl+Z undoes", parent.name);
        return record(cx, ops, move |now| super::toast(now.ed, note));
    }
    // Store the drop point first, so the patched level draws the brick there.
    // A lane drop lets the layout place the brick in its lane.
    let lane = !matches!(&d.dest, Dest::Here(t) if t.is_empty());
    let hint = (!lane).then_some(LayoutHint { x: d.x, y: d.y });
    let req = LayoutRequest {
        brick: layout_key(&lv, &name),
        hint,
    };
    let c = cx.clone();
    let o = Op::AddBrick { file, brick: spec };
    send::<DesignLayout>(cx.ed, req, move |_| {
        record(&c, vec![o], move |now| select_brick(now, &name));
    });
}

/// The inline name box of a brick being placed: Enter adds it where it was dropped.
pub fn draft_box(cx: &Cx, d: Draft) -> View {
    let ed = cx.ed;
    let name = create_signal(String::new());
    let tag = create_signal(String::new());
    let members = create_signal(if d.kind == BrickKind::Module {
        "crate::".to_string()
    } else {
        String::new()
    });
    let c = cx.clone();
    let d2 = d.clone();
    let add = move || {
        // Read the box's signals before closing it (closing disposes them).
        let n = name.get_clone().trim().to_string();
        let (m, t) = (members.get_clone(), tag.get_clone());
        if n.is_empty() || (d2.dest == Dest::NewLane && t.trim().is_empty()) {
            return;
        }
        ed.draft.set(None);
        commit(&c, d2.clone(), n, m, t);
    };
    let enter = move |e: web_sys::KeyboardEvent| match e.key().as_str() {
        "Enter" => add(),
        "Escape" => ed.draft.set(None),
        _ => {}
    };
    let module = (d.kind == BrickKind::Module).then(|| view! { input(class="inmem", r#type="text", title="members: crate::module", bind:value=members) });
    let lane = (d.dest == Dest::NewLane).then(|| view! { input(class="inmem", r#type="text", placeholder="new tag lane", bind:value=tag) });
    let whither = match &d.dest {
        Dest::Here(t) if t.is_empty() => String::new(),
        Dest::Here(t) => format!("tag: {}", t.join(", ")),
        Dest::Into(p) => format!("into {p}"),
        Dest::NewLane => "new tag lane".into(),
    };
    let ph = format!("{} name, Enter", kind_name(d.kind).to_lowercase());
    let style = at(cx, d.x, d.y);
    view! {
        div(class="inbox draft", style=style, on:mousedown=stop, on:click=stop) {
            span(class="whither") { (whither) }
            input(id="inline-name", r#type="text", placeholder=ph, bind:value=name, on:keydown=enter)
            (lane) (module)
        }
    }
}

/// The tag box of brick `name` dropped on the new-lane zone: Enter puts it in that lane.
pub fn lane_box(cx: &Cx, name: String, x: f64, y: f64) -> View {
    let ed = cx.ed;
    let tag = create_signal(String::new());
    let c = cx.clone();
    let enter = move |e: web_sys::KeyboardEvent| match e.key().as_str() {
        "Enter" => {
            let t = tag.get_clone().trim().to_string();
            ed.lane_ask.set(None);
            let file = c.level_file().unwrap_or_default();
            let o = c
                .brick(&name)
                .filter(|_| !t.is_empty())
                .and_then(|b| retag_op(&file, &c.m.g.bricks[b], Some(&t)));
            o.into_iter().for_each(|o| record(&c, vec![o], |_| {}));
        }
        "Escape" => ed.lane_ask.set(None),
        _ => {}
    };
    let style = at(cx, x, y);
    view! {
        div(class="inbox", style=style, on:mousedown=stop, on:click=stop) {
            span(class="whither") { "new tag lane" }
            input(id="inline-name", r#type="text", placeholder="tag, Enter", bind:value=tag, on:keydown=enter)
        }
    }
}
