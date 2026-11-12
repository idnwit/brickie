//! The Inspector tab: the selected rule edge, brick or measured edge with its full editable
//! schema, or the level's properties when nothing is selected.

mod brick;
mod edge;
mod level;
mod review;

pub use review::review_bar;

use crate::state::*;
use sycamore::prelude::*;

pub fn inspector(cx: &Cx) -> View {
    let c = cx.clone();
    let body = move || {
        let rule = c.ed.rule.get_clone();
        let sel = c.eval.with(|ev| ev.sel.clone());
        untrack(|| match (rule, sel) {
            (Some(r), _) => edge::rule_view(&c, r),
            (None, Sel::Brick(b)) if !c.m.g.bricks[b].ghost => brick::brick_view(&c, b),
            (None, Sel::Edge(e)) => edge::measured_view(&c, e),
            _ => level::level_view(&c),
        })
    };
    view! { (body) }
}
