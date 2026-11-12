//! Ctrl+P quick open: fuzzy match over every file of the explorer's trees.

use super::{St, actions};
use crate::logic;
use sycamore::prelude::*;
use web_sys::{KeyboardEvent, MouseEvent};

const LIMIT: usize = 30;

pub fn quick_open(st: St) -> View {
    let input = create_signal(String::new());
    create_effect(move || {
        if let Some(q) = st.quick.get_clone() {
            input.set(q);
        }
    });
    let hits = create_memo(move || {
        let paths = st.views.with(|v| {
            let all: Vec<_> = v.iter().flat_map(|t| t.roots.clone()).collect();
            logic::files(&all)
        });
        logic::quick(&input.get_clone(), &paths, LIMIT)
    });
    let list = move || {
        View::from(
            hits.get_clone()
                .into_iter()
                .enumerate()
                .map(|(i, p)| {
                    let name = logic::file_name(&p).to_string();
                    let (p1, p2) = (p.clone(), p);
                    let cls = if i == 0 { "qi first" } else { "qi" };
                    view! {
                        div(class=cls, on:click=move |_: MouseEvent| actions::open(st, &p1, 0, 0)) {
                            span(class="qn") { (name) } span(class="qp") { (p2) }
                        }
                    }
                })
                .collect::<Vec<_>>(),
        )
    };
    let keys = move |e: KeyboardEvent| match e.key().as_str() {
        "Escape" => st.quick.set(None),
        "Enter" => {
            if let Some(p) = hits.with(|h| h.first().cloned()) {
                actions::open(st, &p, 0, 0);
            }
        }
        _ => {}
    };
    let body = move || {
        if st.quick.with(Option::is_none) {
            return View::default();
        }
        view! {
            div(class="qo-bg", on:click=move |_: MouseEvent| st.quick.set(None)) {
                div(class="qo", on:click=|e: MouseEvent| e.stop_propagation()) {
                    input(id="cv-quick", placeholder="Go to file\u{2026}", bind:value=input, on:keydown=keys)
                    div(class="ql") { (list) }
                }
            }
        }
    };
    view! { (body) }
}
