//! The browser app: state, actions on it, and the panes (explorer, editor, quick open).

mod actions;
mod editor;
mod explorer;
mod js;
mod quick;

use codeview_port::{Location, Mark, View as Tree, Views};
use std::collections::{HashMap, HashSet};
use sycamore::prelude::*;
use wasm_bindgen::prelude::*;

/// A list of locations from F12 / Shift+F12.
#[derive(Clone, PartialEq)]
pub struct Results {
    pub title: String,
    pub items: Vec<Location>,
}

/// Every piece of UI state; signals are `Copy`, so is this.
#[derive(Clone, Copy)]
pub struct St {
    pub views: Signal<Vec<Tree>>,
    /// Index of the view shown in the explorer.
    pub view: Signal<usize>,
    /// Node id the explorer is scoped to.
    pub filter: Signal<Option<String>>,
    /// Expanded explorer node ids.
    pub expanded: Signal<HashSet<String>>,
    pub tabs: Signal<Vec<String>>,
    pub active: Signal<Option<String>>,
    pub marks: Signal<HashMap<String, Vec<Mark>>>,
    pub stale: Signal<HashSet<String>>,
    pub results: Signal<Option<Results>>,
    /// The quick open query, `None` when closed.
    pub quick: Signal<Option<String>>,
    pub status: Signal<String>,
    pub dark: Signal<bool>,
    /// Whether the bands gutter is shown.
    pub bands: Signal<bool>,
}

fn app() -> View {
    let q = crate::logic::Query::parse(&js::search());
    let st = St {
        views: create_signal(Vec::new()),
        view: create_signal(0),
        filter: create_signal(q.filter.clone()),
        expanded: create_signal(HashSet::new()),
        tabs: create_signal(Vec::new()),
        active: create_signal(None),
        marks: create_signal(HashMap::new()),
        stale: create_signal(HashSet::new()),
        results: create_signal(None),
        quick: create_signal(None),
        status: create_signal(String::new()),
        dark: create_signal(js::prefers_dark()),
        bands: create_signal(true),
    };
    create_effect(move || {
        let dark = st.dark.get();
        js::set_dark_class(dark);
        js::theme(dark);
    });
    create_effect(move || js::show_bands(st.bands.get()));
    let want_entry = q.path.is_none();
    js::call::<Views>(&(), move |r| match r {
        Ok(v) => {
            let ix = q
                .view
                .as_ref()
                .and_then(|n| v.iter().position(|t| &t.name == n))
                .unwrap_or(0);
            let ids = [
                st.filter.get_clone_untracked(),
                st.active.get_clone_untracked(),
            ];
            st.expanded.set(actions::first_expanded(&v, &ids));
            st.views.set(v);
            st.view.set(ix);
            if want_entry {
                actions::open_entry(st);
            }
        }
        Err(e) => st.status.set(format!("views: {e}")),
    });
    js::on_message(move |m| actions::message(st, m));
    js::to_host(&codeview_port::ToHost::Ready);
    js::on_ctrl_key(move |k| actions::ctrl_key(st, k));
    let ex = explorer::explorer(st);
    let ed = editor::editor(st);
    let qo = quick::quick_open(st);
    let page = view! { div(class="cv") { (ex) (ed) (qo) } };
    // `#cv-editor` exists once the view is in the document.
    on_mount(move || actions::start_editor(st, q.path, q.line, q.col));
    page
}

/// Browser entry: render the viewer into `#cv`.
#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    let host = js::element("cv").ok_or("no #cv element")?;
    sycamore::render_to(app, &host);
    Ok(())
}
