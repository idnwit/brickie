//! The source popup: an iframe of `/source` (the page draws the complexity tree beside the code).

use crate::state::Click;
use sycamore::prelude::*;

pub fn modal_view(modal: Signal<Option<String>>) -> View {
    let body = move || match modal.get_clone() {
        None => View::default(),
        Some(url) => {
            let open = url.clone();
            view! {
                div(class="modal", on:click=move |_: Click| modal.set(None)) {
                    div(class="win", on:click=move |e: Click| e.stop_propagation()) {
                        div(class="bar") {
                            a(href=open, target="_blank") { "open in a tab" }
                            button(on:click=move |_: Click| modal.set(None)) { "close (Esc)" }
                        }
                        iframe(src=url, title="source")
                    }
                }
            }
        }
    };
    view! { (body) }
}
