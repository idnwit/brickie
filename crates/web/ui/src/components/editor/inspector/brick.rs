//! The brick inspector: state and file, the agent review bar, auto-applying fields (name,
//! description, tags, members, path), Make composite / Unnest / Delete, and what the rules decide
//! now for its links.

use super::super::op;
use super::super::widgets::{chips_field, text_field};
use crate::components::call::call;
use crate::state::*;
use brickie_web_api::design::Op;
use brickie_web_api::{BrickView, DesignLinks, INDEPENDENT_LABEL, INDEPENDENT_TIP, LinksRequest};
use sycamore::prelude::*;
use web_sys::wasm_bindgen::JsCast;

/// Fetch the link options of the brick at `path` into `ed.links`.
fn fetch_links(ed: Edit, path: Vec<String>) {
    ed.links.set(Vec::new());
    call::<DesignLinks>(&LinksRequest { path }, move |r| match r {
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
            "path (composite dir)",
            String::new(),
            false,
            move |p| set(&c, &f, &b0, Field::Path(p)),
        )
    });
    // `independent` is allowed at any level.
    let lib = {
        let (c, f, b0) = (cx.clone(), file.clone(), b.clone());
        view! {
            label(title=INDEPENDENT_TIP) {
                input(r#type="checkbox", id="brick-independent", prop:checked=b.independent, on:change=move |e: web_sys::Event| {
                    let on = e.target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .is_some_and(|i| i.checked());
                    set(&c, &f, &b0, Field::Independent(on))
                })
                (INDEPENDENT_LABEL)
            }
        }
    };
    view! { div(class="form") { (name) (desc) (tags) (members) (path) (lib) } }
}

/// A composite, or a brick just made composite (its level exists, still empty).
fn is_composite(cx: &Cx, b: &BrickView) -> bool {
    let key = layout_key(&cx.level().path, &b.name);
    b.composite()
        || cx
            .m
            .design
            .as_deref()
            .is_some_and(|ch| ch.files.contains_key(&key))
}

fn actions(cx: &Cx, file: String, b: &BrickView) -> View {
    let name = b.name.clone();
    let (c, f, n) = (cx.clone(), file.clone(), name.clone());
    let make_comp = move |_: Click| {
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
    let unnest = move |_: Click| {
        op(
            &c,
            Op::Unnest {
                file: f.clone(),
                name: n.clone(),
            },
        )
    };
    let toggle = if is_composite(cx, b) {
        view! { button(id="unnest", title="Lift the children to this level; the brick stays a leaf", on:click=unnest) { "Unnest" } }
    } else {
        view! { button(id="make-composite", title="Give the brick its own level (brickie.toml in a folder of its name)", on:click=make_comp) { "Make composite" } }
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
                    let a = if o.allowed { "allow" } else { "deny" };
                    let cls = format!("la {a}");
                    let (to, k) = (o.to.clone(), format!("{} {}", o.kind, o.scope));
                    let by = match (&o.rule, o.allowed) {
                        (Some(r), _) => r.clone(),
                        (None, true) => "no rule needed (independent brick)".into(),
                        (None, false) => "no rule allows it".into(),
                    };
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
    if brick.is_group {
        return group_view(cx, b, &brick);
    }
    let lv = cx.level().path.clone();
    fetch_links(cx.ed, brick_path(&lv, &brick.name));
    let Some(file) = cx.level_file() else {
        return view! { p(class="err-text") { "This level is not in the design." } };
    };
    let zoom = is_composite(cx, &brick).then(|| {
        let (c, target) = (cx.clone(), layout_key(&lv, &brick.name));
        view! { button(on:click=move |_: Click| c.go(c.params.with(|p| p.at_level(&target)))) { "Open its level" } }
    });
    let sub = format!("in {file}");
    let title = brick.name.clone();
    let copy = super::copy_ref(brick_ref(&lv, &brick.name));
    let (f, a) = (fields(cx, file.clone(), &brick), actions(cx, file, &brick));
    let now = now_lines(cx.ed);
    view! {
        h2 { (title) (copy) }
        p(class="muted") { (sub) }
        (zoom)
        (f)
        (a)
        p(class="muted") { "Drag a typed port on the brick's right edge onto a sibling to add a rule." }
        (now)
    }
}

fn group_view(cx: &Cx, b: usize, brick: &BrickView) -> View {
    let lv = cx.level().path.clone();
    fetch_links(cx.ed, brick_path(&lv, &brick.name));
    let Some(file) = cx.level_file() else {
        return view! { p(class="err-text") { "This level is not in the design." } };
    };
    let (c, f, b0) = (cx.clone(), file.clone(), brick.clone());
    let name = text_field("group-name", "name", brick.name.clone(), false, move |to| {
        super::super::rename_group(&c, &f, &b0.name, &to)
    });
    let (c, f, b0) = (cx.clone(), file.clone(), brick.clone());
    let desc = text_field(
        "group-desc",
        "description",
        brick.desc.clone().unwrap_or_default(),
        true,
        move |d| {
            if let Some(o) = set_group_field(&f, &b0, Field::Desc(d)) {
                op(&c, o);
            }
        },
    );
    let (c, f, b0) = (cx.clone(), file.clone(), brick.clone());
    let tags = chips_field("group-tag", "tags", brick.tags.clone(), "+ tag", move |t| {
        if let Some(o) = set_group_field(&f, &b0, Field::Tags(t)) {
            op(&c, o);
        }
    });
    let members: Vec<View> = brick
        .group_members
        .iter()
        .enumerate()
        .map(|(mi, m)| {
            let (c1, c2, f, mn) = (cx.clone(), cx.clone(), file.clone(), m.name.clone());
            let lbl = m.name.clone();
            view! {
                div(class="row") {
                    a(href="#", on:click=move |e: Click| { e.prevent_default(); c1.select(Sel::Member(b, mi)); }) { (lbl) }
                    button(class="mini", on:click=move |_: Click| op(&c2, set_brick_group_op(&f, &mn, ""))) { "Ungroup" }
                }
            }
        })
        .collect();
    let members = if members.is_empty() {
        view! { p(class="muted") { "No members yet. Drag an ungrouped brick onto this group box." } }
    } else {
        View::from(members)
    };
    let ed = cx.ed;
    let sub = format!("group in {file}");
    let title = brick.name.clone();
    let copy = super::copy_ref(brick_ref(&lv, &brick.name));
    let now = now_lines(cx.ed);
    view! {
        h2 { (title) (copy) }
        p(class="muted") { (sub) }
        div(class="form") { (name) (desc) (tags) }
        h4 { "Members" }
        (members)
        div(class="row acts") {
            button(class="danger", on:click=move |_: Click| ed.confirm.set(true)) { "Delete\u{2026}" }
        }
        (now)
    }
}

pub fn member_view(cx: &Cx, b: usize, m: usize) -> View {
    let grp = cx.m.g.bricks[b].clone();
    let mem = grp.group_members[m].clone();
    let Some(file) = cx.level_file() else {
        return view! { p(class="err-text") { "This level is not in the design." } };
    };
    let c1 = cx.clone();
    let c2 = cx.clone();
    let mname = mem.name.clone();
    let gname = grp.name.clone();
    let zoom = mem.is_composite.then(|| {
        let c = cx.clone();
        let target = mem.path.clone();
        view! { button(on:click=move |_: Click| c.go(c.params.with(|p| p.at_level(&target)))) { "Open its level" } }
    });
    view! {
        h2 { (mem.name.clone()) }
        p(class="muted") {
            "member of group "
            a(href="#", on:click=move |e: Click| { e.prevent_default(); c1.select(Sel::Brick(b)); }) { (gname) }
        }
        (zoom)
        div(class="row acts") {
            button(on:click=move |_: Click| op(&c2, set_brick_group_op(&file, &mname, ""))) { "Remove from group" }
        }
    }
}
