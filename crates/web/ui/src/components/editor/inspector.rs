//! The inspector: the selected brick's name, description and tags (rename, edit, delete), the
//! links it may draw (pick one, drop it on a sibling) and the way into a group.

use super::send_reload;
use crate::components::call::call;
use crate::state::*;
use brickie_web_api::change::Op;
use brickie_web_api::{BrickView, ChangeLinks, ChangeLinksRequest, ChangeOp, ChangeOpRequest};
use sycamore::prelude::*;

fn op(ed: Edit, op: Op) {
    send_reload::<ChangeOp>(ed, ChangeOpRequest { op, note: None });
}

/// Fetch the link options of the brick at `path` into `ed.links`.
fn fetch_links(ed: Edit, path: Vec<String>) {
    ed.links.set(Vec::new());
    call::<ChangeLinks>(&ChangeLinksRequest { path }, move |r| match r {
        Ok(v) => ed.links.set(v),
        Err(e) => ed.msg.set(e),
    });
}

fn fields(cx: &Cx, file: String, b: &BrickView) -> View {
    let ed = cx.ed;
    let name0 = b.name.clone();
    let name = create_signal(b.name.clone());
    let desc = create_signal(b.desc.clone().unwrap_or_default());
    let tags = create_signal(b.tags.join(", "));
    let (f1, f2, f3, n1, n2, n3) = (
        file.clone(),
        file.clone(),
        file,
        name0.clone(),
        name0.clone(),
        name0,
    );
    let rename = move |_: Click| {
        let to = name.get_clone().trim().to_string();
        if !to.is_empty() && to != n1 {
            op(
                ed,
                Op::RenameBrick {
                    file: f1.clone(),
                    from: n1.clone(),
                    to,
                },
            );
        }
    };
    let save = move |_: Click| {
        op(
            ed,
            Op::SetBrick {
                file: f2.clone(),
                name: n2.clone(),
                desc: Some(desc.get_clone().trim().to_string()),
                tags: Some(split_list(&tags.get_clone())),
                members: None,
                path: None,
            },
        );
    };
    let delete = move |_: Click| {
        op(
            ed,
            Op::RemoveBrick {
                file: f3.clone(),
                name: n3.clone(),
            },
        )
    };
    view! {
        div(class="form") {
            label { "name" }
            div(class="row") { input(r#type="text", bind:value=name) button(on:click=rename) { "Rename" } }
            label { "description" }
            textarea(class="desc", bind:value=desc)
            label { "tags (comma separated)" }
            input(r#type="text", bind:value=tags)
            div(class="row") {
                button(class="go", on:click=save) { "Save" }
                button(class="danger", on:click=delete) { "Delete" }
            }
        }
    }
}

/// One (kind, scope) chip: pressing it picks the link with the chosen action.
fn link_chip(ed: Edit, action: Signal<String>, pick: LinkPick) -> View {
    let label = format!("{} \u{00B7} {}", pick.kind, pick.scope);
    let p2 = pick.clone();
    let cls = move || {
        let on = ed.link.with(|l| {
            l.as_ref()
                .is_some_and(|l| l.kind == p2.kind && l.scope == p2.scope)
        });
        format!("lchip{}", if on { " on" } else { "" })
    };
    let down = move |e: Click| {
        e.prevent_default();
        e.stop_propagation();
        ed.tool.set(None);
        ed.link.set(Some(LinkPick {
            action: action.get_clone(),
            ..pick.clone()
        }));
    };
    view! { button(class=cls, on:mousedown=down) { (label) } }
}

fn links(cx: &Cx, from: String) -> View {
    let ed = cx.ed;
    let action = create_signal("allow".to_string());
    let seg = move |a: &'static str| {
        let cls = move || format!("segb{}", if action.with(|x| x == a) { " on" } else { "" });
        view! { a(class=cls, href="#", on:click=move |e: Click| { e.prevent_default(); action.set(a.into()); }) { (a) } }
    };
    let chips = move || {
        let kinds = ed.links.with(|l| link_kinds(l));
        let items: Vec<View> = kinds
            .into_iter()
            .map(|(kind, scope)| {
                let pick = LinkPick {
                    from: from.clone(),
                    kind,
                    scope,
                    action: String::new(),
                };
                link_chip(ed, action, pick)
            })
            .collect();
        View::from(items)
    };
    let table = move || {
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
    view! {
        h4 { "Links" }
        div(class="row") { "rule to add: " span(class="seg") { (seg("allow")) (seg("deny")) } }
        div(class="lchips") { (chips) }
        p(class="muted") { "Drag a link onto a sibling brick (or pick it, then click the brick)." }
        details { summary { "What the rules decide now" } table(class="tr") { (table) } }
    }
}

fn brick_view(cx: &Cx, b: usize) -> View {
    let brick = cx.m.g.bricks[b].clone();
    let lv = cx.level().path.clone();
    let path = brick_path(&lv, &brick.name);
    fetch_links(cx.ed, path);
    let Some(file) = cx.level_file() else {
        return view! { p(class="err-text") { "This level is not in the preview." } };
    };
    let state = match (brick.planned, brick.proposed) {
        (_, true) => "proposed by the agent (review it in the ops log)",
        (true, _) => "planned: no code yet (publish scaffolds it)",
        _ => "built",
    };
    let zoom = brick.composite().then(|| {
        let (c, target) = (cx.clone(), brick.path.clone());
        view! { button(on:click=move |_: Click| c.go(c.params.with(|p| p.at_level(&target)))) { "Open its level" } }
    });
    let shared = format!("in {file}");
    let title = brick.name.clone();
    let f = fields(cx, file, &brick);
    let l = links(cx, brick.name.clone());
    view! {
        h2 { (title) }
        p(class="muted") { (state) " \u{00B7} " (shared) }
        (zoom)
        (f)
        (l)
    }
}

/// The inspector tab: the selected brick, or a hint.
pub fn inspector(cx: &Cx) -> View {
    let c = cx.clone();
    let body = move || match c.eval.with(|ev| ev.sel.clone()) {
        Sel::Brick(b) if !c.m.g.bricks[b].ghost => untrack(|| brick_view(&c, b)),
        _ => view! {
            p(class="muted") { "Nothing selected. Click a brick to edit it and draw its links; double-click a group to zoom in and add children." }
        },
    };
    view! { (body) }
}
