//! The source popup: an iframe of the code viewer (`/code/`, the `codeview` component) at the
//! linked file, line and column, with the complexity bands of its functions in the gutter.
//! When the viewer says `Ready` it gets `Filter` (the explorer scoped to the selected brick); a
//! finding link clicked in it comes back as `Link` and selects the finding.
//! Without JavaScript a link opens the code viewer in a tab (which itself needs JavaScript).

use crate::platform;
use crate::state::Click;
use brickie_web_api::Params;
use codeview_port::{ToHost, ToViewer};
use sycamore::prelude::*;

const FRAME: &str = "code-frame";

/// The explorer node of the single selected brick (`root/web/server`), if any.
fn brick_node(p: &Params) -> Option<String> {
    match p.bricks().as_slice() {
        [b] => Some(format!("{}/{b}", p.level_path())),
        _ => None,
    }
}

/// The messages sent to the viewer once it has loaded: a file link (`path=`) scopes the
/// explorer to the selected brick; a brick link (`filter=`, the "Open code" button) carries
/// its own scope.
fn greeting(src: &str, p: &Params) -> Vec<ToViewer> {
    if src.contains("filter=") {
        return Vec::new();
    }
    vec![ToViewer::Filter {
        view: "Bricks".into(),
        node: brick_node(p),
    }]
}

/// Handle the viewer's messages: `Ready` gets the greeting, `Link` selects the finding,
/// `Close` closes the popup.
pub fn listen(modal: Signal<Option<String>>, params: Signal<Params>) {
    platform::on_message(move |text| match serde_json::from_str::<ToHost>(&text) {
        Ok(ToHost::Ready) => {
            let Some(src) = modal.get_clone_untracked() else {
                return;
            };
            let msgs = greeting(&src, &params.get_clone_untracked());
            for json in msgs.iter().filter_map(|m| serde_json::to_string(m).ok()) {
                platform::post_to_frame(FRAME, &json);
            }
        }
        Ok(ToHost::Link { href }) => {
            let query = href.split_once('?').map_or("", |(_, q)| q);
            modal.set(None);
            params.set(Params::from_query(query));
        }
        Ok(ToHost::Close) => modal.set(None),
        _ => {}
    });
}

/// The popup window around the viewer at `url` (a code viewer link).
fn window(modal: Signal<Option<String>>, url: String) -> View {
    let open = url.clone();
    // Full screen: the window covers the whole page (toggle; kept while the popup is open).
    let full = create_signal(false);
    let win_cls = move || if full.get() { "win full" } else { "win" };
    let full_label = move || {
        if full.get() {
            "\u{2921} exit full screen"
        } else {
            "\u{26F6} full screen"
        }
    };
    view! {
        div(class="modal", on:click=move |_: Click| modal.set(None)) {
            div(class=win_cls, on:click=move |e: Click| e.stop_propagation()) {
                div(class="bar") {
                    a(href=open, target="_blank") { "open in a tab" }
                    span(class="acts") {
                        button(title="Toggle full screen", on:click=move |_: Click| full.set(!full.get())) { (full_label) }
                        button(on:click=move |_: Click| modal.set(None)) { "close (Esc)" }
                    }
                }
                iframe(id=FRAME, src=url, title="code")
            }
        }
    }
}

pub fn modal_view(modal: Signal<Option<String>>) -> View {
    let body = move || match modal.get_clone() {
        None => View::default(),
        Some(src) => window(modal, src),
    };
    view! { (body) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_viewer_is_scoped_to_the_selected_brick() {
        let p = Params::from_query("lv=root/web&brick=server");
        let src = brickie_web_api::code_href("a.rs", 3, Some(7));
        assert_eq!(
            greeting(&src, &p),
            vec![ToViewer::Filter {
                view: "Bricks".into(),
                node: Some("root/web/server".into())
            }]
        );
        assert_eq!(
            brickie_web_api::code_brick_href("root/web/server"),
            "/code/?filter=root/web/server&view=Bricks"
        );
        assert!(greeting("/code/?filter=root/a&view=Bricks", &p).is_empty());
        assert_eq!(brick_node(&Params::from_query("brick=a,b")), None);
    }
}
