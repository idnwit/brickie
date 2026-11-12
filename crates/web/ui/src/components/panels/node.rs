//! The tree part of a brick's details: a composite's config, exports and children (a
//! double-click on the brick zooms in), a faded neighbour's place outside the level, or a
//! leaf's members.

use crate::model::BrickView;
use crate::state::*;
use sycamore::prelude::*;

/// The brick's one-line description (`desc`), shown under its name; empty when it has none.
pub fn desc(br: &BrickView) -> View {
    match br.desc.clone() {
        Some(d) => view! { p(class="desc") { (d) } },
        None => View::default(),
    }
}

pub fn tree_info(cx: &Cx, br: &BrickView) -> View {
    if br.ghost {
        let up = br
            .path
            .rsplit_once('/')
            .map_or("root", |(a, _)| a)
            .to_string();
        let path = br.path.clone();
        let open = nav_link(
            cx,
            format!("open {up}"),
            "chip",
            |_| false,
            move |p| p.at_level(&up),
        );
        return view! {
            h4 { "Outside this level" }
            div(class="mono") { (path) }
            div(class="muted small") { "Drawn faded: it has edges crossing the boundary of this level." }
            (open)
        };
    }
    if !br.composite() {
        let members: Vec<View> = br
            .members
            .iter()
            .cloned()
            .map(|m| view! { div(class="mono") { (m) } })
            .collect();
        let members = View::from(members);
        return view! { h4 { "Members" } (members) };
    }
    let kids: Vec<View> = br
        .children
        .iter()
        .map(|k| {
            let text = if br.exports.contains(k) {
                format!("{k} \u{25CF} exported")
            } else {
                k.clone()
            };
            view! { div(class="mono") { (text) } }
        })
        .collect();
    let kids = View::from(kids);
    let file = br.file.clone().unwrap_or_default();
    let exports = format!("exports [{}]", br.exports.join(", "));
    let n_kids = format!("Children ({})", br.children.len());
    view! {
        h4 { "Composite" }
        div(class="mono small") { (file) }
        div(class="mono small") { (exports) }
        h4 { (n_kids) }
        (kids)
    }
}
