//! The page: the original viewer's shell (header with presets and data layers, findings list,
//! canvas, detail panel) for the level on screen. Rendered on the server and hydrated in the
//! browser; every state change goes through [`Params`] (the URL). Changing the level rebuilds
//! the shell from a fresh [`Cx`] (its own layout, pan/zoom and drag state).

use super::canvas::{canvas, pointer};
use super::controls::header;
use super::editor;
use super::keys::keys;
use super::panels::{detail, left};
use super::popup::{listen, modal_view};
use crate::platform;
use crate::state::*;
use brickie_web_api::*;
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
    // While a change is open its bar takes the banner's place.
    let ban = match cx.m.change {
        Some(_) => editor::change_bar(cx),
        None => banner(&cx.m.cache),
    };
    let nav = left(cx);
    let stage = canvas(cx);
    let det = if cx.editing() {
        editor::panel(cx)
    } else {
        detail(cx)
    };
    view! { (hdr) (ban) (nav) (stage) (det) }
}

pub fn app(data: PageData, initial: Params) -> View {
    let sh = Shared::new(data, initial);
    let cur: Cur = Rc::new(RefCell::new(None));
    {
        let params = sh.params;
        // The URL last seen or written. Params changes that differ from it are user navigation
        // and push a history entry; back/forward (popstate) records its URL first, so it does not.
        let url = Rc::new(RefCell::new(params.with(|p| p.url())));
        let u = url.clone();
        platform::on_popstate(move || {
            let p = Params::from_query(&platform::search());
            *u.borrow_mut() = p.url();
            params.set(p);
        });
        create_effect(move || {
            let now = params.with(|p| p.url());
            if *url.borrow() != now {
                platform::push_url(&now);
                *url.borrow_mut() = now;
            }
        });
        let c = cur.clone();
        platform::on_resize(move || {
            if let (Some(c), Some(r)) = (current(&c), platform::stage_rect()) {
                c.stage.set(Some((r.2, r.3)));
                c.view.set(fit_view(&c.m.layout, (r.2, r.3)));
            }
        });
        keys(&cur);
        pointer(&cur);
        editor::start_poll();
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
            // An editor patch re-renders the level in place (pan/zoom kept).
            sh.live.rev.track();
            untrack(|| {
                let depth = sh.live.levels.with(|l| l[lv].path.matches('/').count());
                let anim = match prev.replace(Some(depth)) {
                    Some(d) if depth > d => "zin",
                    Some(d) if depth < d => "zout",
                    _ => "",
                };
                let mut cx = Cx::new(&sh, lv, platform::stage_rect().map(|r| (r.2, r.3)));
                cx.anim = anim;
                *cur.borrow_mut() = Some(cx.clone());
                set_current_cx(&cx);
                page(&cx)
            })
        }
    };
    listen(sh.modal, sh.params);
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
