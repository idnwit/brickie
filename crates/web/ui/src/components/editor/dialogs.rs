//! The change bar's dialogs: open a change (intent), the publish plan (every file publish
//! writes, and why it would refuse) and discard.

use super::{send, send_reload};
use crate::components::call::get;
use crate::platform;
use crate::state::*;
use brickie_web_api::change::{Plan, PlanAction};
use brickie_web_api::*;
use sycamore::prelude::*;

/// Fetch the publish plan into the open dialog.
pub fn load_plan(ed: Edit) {
    get::<ChangePublishPlan>(move |r| match r {
        Ok(p) => ed.dialog.set(Dialog::Publish(Some(p))),
        Err(e) => {
            ed.dialog.set(Dialog::None);
            ed.msg.set(e);
        }
    });
}

fn action(a: PlanAction) -> &'static str {
    match a {
        PlanAction::Write => "write",
        PlanAction::Create => "create",
        PlanAction::Edit => "edit",
        PlanAction::Mkdir => "mkdir",
        PlanAction::Skip => "skip",
    }
}

fn plan_view(ed: Edit, plan: Plan) -> View {
    let rows: Vec<View> = plan
        .files
        .iter()
        .map(|f| {
            let (a, path, note) = (action(f.action), f.path.clone(), f.note.clone());
            view! { tr { td(class=format!("pa {a}")) { (a) } td(class="mono") { (path) } td(class="muted") { (note) } } }
        })
        .collect();
    let blockers: Vec<View> = plan
        .blockers
        .iter()
        .map(|b| {
            let b = b.clone();
            view! { li(class="err-text") { (b) } }
        })
        .collect();
    let blocked = !plan.blockers.is_empty();
    let (rows, blockers) = (View::from(rows), View::from(blockers));
    let publish = move |_: Click| {
        send::<ChangePublish>(ed, (), |_| platform::goto("/"));
    };
    view! {
        h3 { "Publish" }
        p(class="muted") { "Publish writes these files (existing files are never overwritten), then closes the change." }
        table(class="plan") { (rows) }
        ul { (blockers) }
        div(class="acts") {
            button(on:click=move |_: Click| ed.dialog.set(Dialog::None)) { "Cancel" }
            button(class="go", disabled=blocked, on:click=publish) { "Publish" }
        }
    }
}

fn open_view(ed: Edit) -> View {
    let intent = create_signal(String::new());
    let open = move |_: Click| {
        let req = ChangeOpenRequest {
            intent: intent.get_clone(),
        };
        send::<ChangeOpen>(ed, req, |_| {
            let p = Params::from_query(&platform::search());
            platform::goto(&p.with(|p| p.change = "1".into()).url());
        });
    };
    view! {
        h3 { "New design change" }
        p(class="muted") { "While the change is open, nobody writes brickie.toml: you and the coding agent record ops; publish writes them." }
        input(id="intent", r#type="text", placeholder="intent: what is this change for?", bind:value=intent)
        div(class="acts") {
            button(on:click=move |_: Click| ed.dialog.set(Dialog::None)) { "Cancel" }
            button(class="go", disabled=move || intent.with(|i| i.trim().is_empty()), on:click=open) { "Open change" }
        }
    }
}

fn discard_view(ed: Edit) -> View {
    view! {
        h3 { "Discard the change?" }
        p(class="muted") { "Every op of the change is deleted; brickie.toml stays as it is." }
        div(class="acts") {
            button(on:click=move |_: Click| ed.dialog.set(Dialog::None)) { "Cancel" }
            button(class="danger", on:click=move |_: Click| send_reload::<ChangeDiscard>(ed, ())) { "Discard" }
        }
    }
}

/// The modal of the change bar.
pub fn dialog(ed: Edit) -> View {
    let body = move || {
        let inner = match ed.dialog.get_clone() {
            Dialog::None => return View::default(),
            Dialog::Open => open_view(ed),
            Dialog::Discard => discard_view(ed),
            Dialog::Publish(None) => view! { p { "Loading the publish plan\u{2026}" } },
            Dialog::Publish(Some(plan)) => plan_view(ed, plan),
        };
        let err = move || ed.msg.get_clone();
        view! {
            div(class="edmodal") {
                div(class="edbox", on:click=|e: Click| e.stop_propagation()) {
                    (inner)
                    p(class="err-text") { (err) }
                }
            }
        }
    };
    view! { (body) }
}
