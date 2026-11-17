//! The left panel in Design mode: only what the design files declare (Design mode reads no
//! code). The level's bricks with their tags and members as written, its exports and its
//! links; no findings, scores, functions or files.

use crate::state::*;
use sycamore::prelude::*;

fn brick_row(b: &brickie_web_api::BrickView) -> View {
    let name = b.name.clone();
    let tags = (!b.tags.is_empty()).then(|| {
        let t = b.tags.join(", ");
        view! { span(class="muted") { " [" (t) "]" } }
    });
    let flag = b
        .independent
        .then(|| view! { span(class="pill") { "independent" } });
    let members = b.members.join(", ");
    view! {
        li(class="dbrick") {
            b { (name) } (tags) " " (flag)
            div(class="small mono muted") { (members) }
        }
    }
}

fn link_row(e: &brickie_web_api::LevelLink) -> View {
    let text = format!("{} \u{2192} {}", e.from, e.to);
    let (reason, cls) = (
        e.reason.clone(),
        if e.denied.is_some() {
            "dlink denied"
        } else {
            "dlink"
        },
    );
    let why = e
        .denied
        .clone()
        .map(|w| view! { div(class="small err-text") { "not allowed: " (w) } });
    view! { li(class=cls) { b { (text) } " " span(class="muted") { (reason) } (why) } }
}

/// `nav#left` in Design mode.
pub fn design_left(cx: &Cx) -> View {
    let lv = cx.level();
    let bricks: Vec<View> = (cx.m.g.bricks.iter())
        .filter(|b| !b.ghost && !b.unassigned)
        .map(brick_row)
        .collect();
    let links: Vec<View> = lv.links.iter().map(link_row).collect();
    let exports = (!lv.exports.is_empty()).then(|| {
        let e = lv.exports.join(", ");
        view! { h3(class="idx") { "Exports" } p(class="mono") { (e) } }
    });
    let no_links = links
        .is_empty()
        .then(|| view! { p(class="muted") { "No links at this level." } });
    let (bricks, links) = (View::from(bricks), View::from(links));
    view! {
        nav(id="left", class="design") {
            h3(class="idx") { "Bricks (as declared)" }
            ul(class="dlist") { (bricks) }
            (exports)
            h3(class="idx") { "Links" }
            ul(class="dlist") { (links) }
            (no_links)
        }
    }
}
