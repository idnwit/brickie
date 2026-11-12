//! The editor pane: tabs, the Monaco host element, and the bottom panel with the active
//! file's marks and the last navigation results.

use super::{St, actions, js};
use crate::logic;
use codeview_port::{Level, Mark};
use sycamore::prelude::*;
use web_sys::MouseEvent;

fn tabs(st: St) -> View {
    let list = move || {
        View::from(
            st.tabs
                .get_clone()
                .into_iter()
                .map(|p| {
                    let (p1, p2, p3) = (p.clone(), p.clone(), p.clone());
                    let name = logic::file_name(&p).to_string();
                    let cls = move || {
                        let on = st.active.with(|a| a.as_deref() == Some(p1.as_str()));
                        if on { "tab on" } else { "tab" }
                    };
                    view! {
                        div(class=cls, title=p.clone(), on:click=move |_: MouseEvent| actions::activate(st, &p2)) {
                            span { (name) }
                            button(class="x", title="close", on:click=move |e: MouseEvent| {
                                e.stop_propagation();
                                actions::close(st, &p3);
                            }) { "\u{00d7}" }
                        }
                    }
                })
                .collect::<Vec<_>>(),
        )
    };
    let cls = move || if st.bands.get() { "bt on" } else { "bt" };
    view! {
        div(class="tabs") {
            (list)
            button(class=cls, title="Show or hide the bands gutter (complexity bars)",
                on:click=move |_: MouseEvent| st.bands.set(!st.bands.get())) { "bands" }
        }
    }
}

fn level(l: Level) -> &'static str {
    match l {
        Level::Error => "err",
        Level::Warn => "warn",
        Level::Info => "info",
    }
}

fn mark_row(st: St, path: String, m: Mark) -> View {
    let (line, col) = (m.span.start.line, m.span.start.col);
    let at = format!("{line}:{col}");
    let cls = format!("mk {}", level(m.level));
    let link = match m.link {
        Some(href) => {
            let tip = href.clone();
            view! {
            a(class="lnk", href="#", title=tip, on:click=move |e: MouseEvent| {
                e.prevent_default();
                e.stop_propagation();
                js::follow(&href);
            }) { "finding \u{2197}" }
            }
        }
        None => View::default(),
    };
    let msg = m.message;
    view! {
        div(class=cls, on:click=move |_: MouseEvent| actions::open(st, &path, line, col)) {
            span(class="at") { (at) } span(class="msg") { (msg) } (link)
        }
    }
}

fn panel(st: St) -> View {
    let marks = move || {
        let Some(path) = st.active.get_clone() else {
            return View::default();
        };
        let list = st.marks.with(|m| m.get(&path).cloned().unwrap_or_default());
        let stale = st.stale.with(|s| s.contains(&path));
        let mut rows: Vec<View> = Vec::new();
        if stale {
            rows.push(view! { div(class="mk warn") { "this file changed since it was measured: marks and navigation may be off" } });
        }
        rows.extend(list.into_iter().map(|m| mark_row(st, path.clone(), m)));
        View::from(rows)
    };
    let results = move || match st.results.get_clone() {
        None => View::default(),
        Some(r) => {
            let rows: Vec<View> = r
                .items
                .into_iter()
                .map(|l| {
                    let s = l.span.start;
                    let at = format!("{}:{}", l.path, s.line);
                    let label = l.label;
                    view! {
                        div(class="rs", on:click=move |_: MouseEvent| actions::open(st, &l.path, s.line, s.col)) {
                            span(class="lb") { (label) } span(class="at") { (at) }
                        }
                    }
                })
                .collect();
            let title = r.title;
            let rows = View::from(rows);
            view! {
                div(class="res") {
                    div(class="rh") { span { (title) } button(on:click=move |_: MouseEvent| st.results.set(None)) { "\u{00d7}" } }
                    (rows)
                }
            }
        }
    };
    let status = move || st.status.get_clone();
    view! {
        div(class="panel") { (results) div(class="marks") { (marks) } div(class="status") { (status) } }
    }
}

pub fn editor(st: St) -> View {
    let t = tabs(st);
    let p = panel(st);
    let empty = move || {
        if st.active.with(Option::is_none) {
            view! { div(class="empty") { "Open a file from the explorer, or press Ctrl+P. F12: definition, Shift+F12: references." } }
        } else {
            View::default()
        }
    };
    view! {
        main(class="ed") {
            (t)
            div(class="host") { div(id="cv-editor") (empty) }
            (p)
        }
    }
}
