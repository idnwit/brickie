//! The tag captions at the left of each row: click selects the row (shift adds).

use crate::state::*;
use sycamore::prelude::*;

/// The tag captions at the left of each row: click selects the row (shift adds).
pub fn row_labels(cx: &Cx) -> View {
    let l = &cx.m.layout;
    let items: Vec<View> = l
        .rows
        .iter()
        .map(|r| {
            let (x, y) = (l.px0 - 16.0, r.y + r.h / 2.0);
            let tr = format!("rotate(-90 {x} {y})");
            // Longer labels are cut to the row height (the tooltip has the full tag).
            let max = ((r.h / 11.0) as usize).clamp(6, 14);
            let label = if r.label.chars().count() > max {
                format!("{}\u{2026}", r.label.chars().take(max - 1).collect::<String>())
            } else {
                r.label.clone()
            };
            // Counter the zoom so the caption stays about 12 px on screen.
            let c0 = cx.clone();
            let size = move || format!("font-size:{:.1}px", (12.0 / c0.view.get().k).clamp(10.0, 26.0));
            let (tip, tag) = (r.label.clone(), r.label.clone());
            let c = cx.clone();
            let click = move |e: Click| {
                e.stop_propagation();
                let sel = c.eval.with(|ev| ev.sel.clone());
                c.select(sel.select_tag(&tag, e.shift_key()));
            };
            view! {
                text(x=x.to_string(), y=y.to_string(), class="rcap gsel", style=size, transform=tr, "text-anchor"="middle", on:click=click) {
                    svg_title { (tip) }
                    (label)
                }
            }
        })
        .collect();
    View::from(items)
}
