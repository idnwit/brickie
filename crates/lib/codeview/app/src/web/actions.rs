//! What the viewer does: open files, navigate, and react to the host page and the keyboard.

use super::{Results, St, js};
use crate::logic;
use codeview_port::{
    AtReq, Bands, Definition, Endpoint, File, Location, Marks, Node, PathReq, Pos, References,
    ToHost, ToViewer, View,
};
use std::collections::HashSet;

/// The node ids to expand: the roots, and the paths down to the nodes `ids` (the filtered
/// node, the open file).
pub fn first_expanded(views: &[View], ids: &[Option<String>]) -> HashSet<String> {
    let mut out = HashSet::new();
    for v in views {
        out.extend(v.roots.iter().map(|r| r.id.clone()));
        for id in ids.iter().flatten() {
            ancestors(&v.roots, id, &mut out);
        }
    }
    out
}

/// Insert the ids from the roots down to `id` (inclusive); whether `id` is below `nodes`.
fn ancestors(nodes: &[Node], id: &str, out: &mut HashSet<String>) -> bool {
    for n in nodes {
        if n.id == id || ancestors(&n.children, id, out) {
            out.insert(n.id.clone());
            return true;
        }
    }
    false
}

/// Create the editor (after the first render) and open the file from the URL.
pub fn start_editor(st: St, path: Option<String>, line: Option<u32>, col: Option<u32>) {
    let def = js::callback2(move |l, c| navigate::<Definition>(st, l, c));
    let refs = js::callback2(move |l, c| navigate::<References>(st, l, c));
    let quick = js::callback0(move || {
        ctrl_key(st, "p");
    });
    if let Some(el) = js::element("cv-editor") {
        js::init(&el, st.dark.get_untracked(), &def, &refs, &quick);
    }
    if let Some(p) = path {
        open(st, &p, line.unwrap_or(0), col.unwrap_or(0));
    }
}

/// Open the entry file (crate root, `mod.rs`, else the first file) of the explorer's filtered
/// node, when there is one.
pub fn open_entry(st: St) {
    let Some(f) = st.filter.get_clone_untracked() else {
        return;
    };
    let roots = st.views.with_untracked(|v| {
        v.get(st.view.get_untracked())
            .map(|t| logic::scoped(&t.roots, Some(&f)))
            .unwrap_or_default()
    });
    if let Some(p) = logic::entry_file(&logic::files(&roots)) {
        open(st, &p, 0, 0);
    }
}

/// Open `path` in a tab and show `line` / byte `col` (0: keep the position).
pub fn open(st: St, path: &str, line: u32, col: u32) {
    let views = st.views.get_clone_untracked();
    st.expanded.update(|e| {
        for v in &views {
            ancestors(&v.roots, path, e);
        }
    });
    st.tabs
        .set(logic::open_tab(&st.tabs.get_clone_untracked(), path));
    st.active.set(Some(path.to_string()));
    st.quick.set(None);
    js::to_host(&ToHost::Opened {
        path: path.to_string(),
    });
    if js::has(path) {
        js::show(path, line, col);
        return;
    }
    let p = path.to_string();
    js::call::<File>(&PathReq { path: p.clone() }, move |r| match r {
        Ok(f) => {
            js::load(&f.path, &f.text, &f.lang);
            if f.stale {
                st.stale.update(|s| {
                    s.insert(f.path.clone());
                });
            }
            if st.active.get_clone_untracked().as_deref() == Some(&f.path) {
                js::show(&f.path, line, col);
            }
            load_bands(f.path.clone());
            load_marks(st, f.path);
        }
        Err(e) => {
            st.status.set(format!("{p}: {e}"));
            close(st, &p);
        }
    });
}

fn load_marks(st: St, path: String) {
    js::call::<Marks>(&PathReq { path: path.clone() }, move |r| {
        let marks = r.unwrap_or_default();
        js::set_marks(&path, &serde_json::to_string(&marks).unwrap_or_default());
        st.marks.update(|m| {
            m.insert(path, marks);
        });
    });
}

fn load_bands(path: String) {
    js::call::<Bands>(&PathReq { path: path.clone() }, move |r| {
        let bands = r.unwrap_or_default();
        js::set_bands(&path, &serde_json::to_string(&bands).unwrap_or_default());
    });
}

pub fn activate(st: St, path: &str) {
    st.active.set(Some(path.to_string()));
    js::show(path, 0, 0);
}

pub fn close(st: St, path: &str) {
    let (tabs, next) = logic::close_tab(&st.tabs.get_clone_untracked(), path);
    st.tabs.set(tabs);
    js::dispose(path);
    if st.active.get_clone_untracked().as_deref() == Some(path) {
        st.active.set(next.clone());
        if let Some(n) = next {
            js::show(&n, 0, 0);
        }
    }
}

/// F12 / Shift+F12 at `(line, col)` of the active file: one definition opens directly,
/// anything else is listed.
fn navigate<E>(st: St, line: u32, col: u32)
where
    E: Endpoint<Req = AtReq, Res = Vec<Location>> + 'static,
{
    let Some(path) = st.active.get_clone_untracked() else {
        return;
    };
    let refs = E::PATH == References::PATH;
    let req = AtReq {
        path,
        pos: Pos { line, col },
    };
    js::call::<E>(&req, move |r| match r {
        Ok(items) if !refs && items.len() == 1 => {
            let s = items[0].span.start;
            open(st, &items[0].path, s.line, s.col);
        }
        Ok(items) => {
            let what = if refs { "References" } else { "Definitions" };
            st.results.set(Some(Results {
                title: format!("{what} ({}) at line {line}", items.len()),
                items,
            }));
        }
        Err(e) => st.status.set(e),
    });
}

/// A message from the host page.
pub fn message(st: St, m: ToViewer) {
    match m {
        ToViewer::Open { path, span } => {
            let s = span.map(|s| s.start).unwrap_or(Pos { line: 0, col: 0 });
            open(st, &path, s.line, s.col);
        }
        ToViewer::Filter { view, node } => {
            if let Some(i) = st.views.with(|v| v.iter().position(|t| t.name == view)) {
                st.view.set(i);
            }
            let more = first_expanded(&st.views.get_clone_untracked(), std::slice::from_ref(&node));
            st.expanded.update(|e| e.extend(more));
            st.filter.set(node);
        }
        ToViewer::Theme { dark } => st.dark.set(dark),
    }
}

/// Ctrl+P opens quick open (Monaco's own Ctrl+F stays).
pub fn ctrl_key(st: St, key: &str) -> bool {
    if key == "p" || key == "P" {
        st.quick.set(Some(String::new()));
        js::focus("cv-quick");
        return true;
    }
    false
}
