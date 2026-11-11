//! The page: the original viewer's shell (header with presets and data layers, findings list,
//! canvas, detail panel) built from one [`Cx`]. Rendered on the server and hydrated in the
//! browser; every state change goes through [`Params`] (the URL).

use super::canvas::canvas;
use super::controls::header;
use super::panels::{current_list, detail, left};
use super::source::modal_view;
use crate::model::*;
use crate::platform;
use crate::state::*;
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

pub fn app(data: PageData, initial: Params) -> View {
    let cx = Cx::new(data, initial);
    {
        let c = cx.clone();
        platform::on_popstate(move || c.params.set(Params::from_query(&platform::search())));
        let c = cx.clone();
        platform::on_resize(move || {
            if let Some(r) = platform::stage_rect() {
                c.stage.set(Some((r.2, r.3)));
                c.view.set(fit_view(&c.m.layout, (r.2, r.3)));
            }
        });
        let c = cx.clone();
        platform::on_key(move |key, in_input| {
            if in_input {
                return false;
            }
            let p = c.params.get_clone();
            match key.as_str() {
                "1" | "2" | "3" => {
                    let name = ["arch", "viol", "all"][key.as_bytes()[0] as usize - b'1' as usize];
                    c.go(with_preset(&p, name));
                }
                "Escape" => {
                    if c.modal.get_clone().is_some() {
                        c.modal.set(None);
                    } else if !p.item.is_empty() {
                        c.go(p.with(|p| p.item.clear()));
                    } else {
                        c.select(Sel::None);
                    }
                }
                "/" => {
                    platform::focus("q");
                    return true;
                }
                "j" | "k" => {
                    let list = current_list(&c);
                    if list.is_empty() {
                        return false;
                    }
                    let cur = list.iter().position(|&i| c.m.diags[i].id == p.item);
                    let next = if key == "j" {
                        cur.map_or(0, |i| (i + 1).min(list.len() - 1))
                    } else {
                        cur.map_or(0, |i| i.saturating_sub(1))
                    };
                    let id = c.m.diags[list[next]].id.clone();
                    c.go(p.with(|p| p.item = id));
                }
                _ => {}
            }
            false
        });
    }
    let hdr = header(&cx);
    let ban = banner(&cx.m.cache);
    let nav = left(&cx);
    let stage = canvas(&cx);
    let det = detail(&cx);
    let popup = modal_view(cx.modal);
    let c = cx.clone();
    // A click anywhere closes an open dropdown.
    let wrap = move |_: Click| c.menu.set(String::new());
    view! {
        div(class="viewer", on:click=wrap) {
            (hdr)
            (ban)
            (nav)
            (stage)
            (det)
            (popup)
        }
    }
}
