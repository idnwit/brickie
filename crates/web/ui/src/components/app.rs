//! The page: the original viewer's shell (header with presets and data layers, findings list,
//! canvas, detail panel) for the level on screen. Rendered on the server and hydrated in the
//! browser; every state change goes through [`Params`] (the URL). Changing the level rebuilds
//! the shell from a fresh [`Cx`] (its own layout, pan/zoom and drag state).

use super::canvas::{canvas, pointer};
use super::controls::header;
use super::keys::keys;
use super::panels::{detail, left};
use super::source::modal_view;
use crate::model::*;
use crate::platform;
use crate::state::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use sycamore::prelude::*;

/// The last three components of a path (`.../a/b/c`); the full path is the tooltip.
fn short_path(p: &str) -> String {
    let parts: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 3 {
        p.to_string()
    } else {
        format!("\u{2026}/{}", parts[parts.len() - 3..].join("/"))
    }
}

fn banner(cache: &CacheInfo) -> View {
    let commit = match &cache.commit {
        Some(c) => format!(
            "commit {}{}",
            c.get(..8).unwrap_or(c),
            if cache.dirty { " (dirty)" } else { "" }
        ),
        None => "no git".to_string(),
    };
    let text = format!(
        "{} \u{00B7} {commit} \u{00B7} measured {}",
        short_path(&cache.workspace),
        cache.measured
    );
    let tip = cache.workspace.clone();
    view! { div(id="banner", class="info", title=tip) { (text) } }
}

/// The shell of one level.
fn page(cx: &Cx) -> View {
    let hdr = header(cx);
    let ban = banner(&cx.m.cache);
    let nav = left(cx);
    let stage = canvas(cx);
    let det = detail(cx);
    view! { (hdr) (ban) (nav) (stage) (det) }
}

pub fn app(data: PageData, initial: Params) -> View {
    let sh = Shared::new(data, initial);
    let cur: Cur = Rc::new(RefCell::new(None));
    {
        let params = sh.params;
        platform::on_popstate(move || params.set(Params::from_query(&platform::search())));
        let c = cur.clone();
        platform::on_resize(move || {
            if let (Some(c), Some(r)) = (current(&c), platform::stage_rect()) {
                c.stage.set(Some((r.2, r.3)));
                c.view.set(fit_view(&c.m.layout, (r.2, r.3)));
            }
        });
        keys(&cur);
        pointer(&cur);
    }
    let lvl = {
        let sh = sh.clone();
        create_selector(move || sh.params.with(|p| sh.level_of(p)))
    };
    // Depth of the previous level: zooming in or out picks the transition.
    let prev: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
    let body = {
        let (sh, cur) = (sh.clone(), cur.clone());
        move || {
            let lv = lvl.get();
            untrack(|| {
                let depth = sh.levels[lv].path.matches('/').count();
                let anim = match prev.replace(Some(depth)) {
                    Some(d) if depth > d => "zin",
                    Some(d) if depth < d => "zout",
                    _ => "",
                };
                let mut cx = Cx::new(&sh, lv);
                cx.anim = anim;
                *cur.borrow_mut() = Some(cx.clone());
                page(&cx)
            })
        }
    };
    let popup = modal_view(sh.modal);
    let menu = sh.menu;
    // A click anywhere closes an open dropdown.
    let wrap = move |_: Click| menu.set(String::new());
    view! {
        div(class="viewer", on:click=wrap) {
            (body)
            (popup)
        }
    }
}
