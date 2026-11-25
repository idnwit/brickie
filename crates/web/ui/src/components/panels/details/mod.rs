//! The right panel (`renderDetail`): overview, brick, edge or finding details, then the
//! scoped functions table and the legend.

mod brick;
mod iface;
mod item;
mod links;
mod summary;

use super::{functions::functions_view, legend::legend};
use crate::state::*;
use brick::brick_detail;
use item::{edge_detail, finding_detail};
use summary::summary;
use sycamore::prelude::*;

/// `renderDetail`.
pub fn detail(cx: &Cx) -> View {
    let content = {
        let cx = cx.clone();
        move || {
            let item = cx.params.with(|p| p.item.clone());
            if let Some(i) =
                cx.m.diags
                    .iter()
                    .position(|d| d.id == item)
                    .filter(|_| !item.is_empty())
            {
                return finding_detail(&cx, i);
            }
            match cx.eval.with(|ev| ev.sel.clone()) {
                Sel::None => summary(&cx),
                Sel::Brick(b) => brick_detail(&cx, b),
                Sel::Member(b, m) => brick::member_detail(&cx, b, m),
                Sel::Edge(e) => edge_detail(&cx, e),
            }
        }
    };
    // A link scopes the lists to itself: the Functions table is hidden then.
    let fns = {
        let cx = cx.clone();
        move || {
            if cx.eval.with(|ev| matches!(ev.sel, Sel::Edge(_))) {
                View::default()
            } else {
                functions_view(&cx)
            }
        }
    };
    let leg = legend(cx);
    view! {
        aside(id="detail") {
            div(id="dcont") { (content) (fns) }
            (leg)
        }
    }
}
