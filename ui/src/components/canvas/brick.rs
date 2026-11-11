//! One brick: box, name, heat bar, tag line and the score badge (`renderGraph` node part).

use crate::layout::Rect;
use crate::model::{fmt_score, heat_len, heat_level};
use crate::state::*;
use sycamore::prelude::*;

/// One brick: box, name, heat bar, tag line and the score badge (`renderGraph` node part).
pub fn node_view(cx: &Cx, b: usize, pos: ReadSignal<Vec<Rect>>) -> View {
    let brick = cx.m.g.bricks[b].clone();
    let r0 = cx.m.layout.pos[b];
    let tip = {
        let (cx, name, tag) = (cx.clone(), brick.name.clone(), brick.tag.clone());
        move || {
            let s = cx.scores.with(|s| s[b].clone());
            format!(
                "{name} [{tag}]\nscore {} ({}E {}W, complexity excess {})",
                fmt_score(s.score),
                s.errors,
                s.warns,
                fmt_score(s.excess)
            )
        }
    };
    let cls = {
        let cx = cx.clone();
        move || {
            let mut c = String::from("node");
            cx.eval.with(|ev| {
                let vio = cx.params.with(|p| p.preset() == "viol");
                let (n, err) = ev.node[b];
                let off_focus = ev.focus.as_ref().is_some_and(|f| !f.lit.contains(&b));
                if off_focus || (vio && n == 0) {
                    c += " dh";
                }
                let item = cx.params.with(|p| !p.item.is_empty());
                let selected = if item {
                    ev.focus
                        .as_ref()
                        .is_some_and(|f| f.finding && f.bricks.contains(&b))
                } else {
                    ev.sel == Sel::Brick(b)
                };
                if selected {
                    c += " sel";
                }
                if n > 0 {
                    c += if err { " bd-red" } else { " bd-amber" };
                } else if ev.node_acc[b] {
                    c += " acc";
                }
            });
            if let Some(h) = cx.hover.get()
                && finding_focus(&cx.m.g, &cx.m.diags[h]).bricks.contains(&b)
            {
                c += " hov";
            }
            c
        }
    };
    let transform = move || pos.with(|p| format!("translate({},{})", p[b].x, p[b].y));
    // Corner badge: `3E 1W` of the open errors and warnings, a green check when clean.
    let badge = {
        let cx = cx.clone();
        move || {
            let (e, w) = cx.scores.with(|s| (s[b].errors, s[b].warns));
            let (label, cls) = match (e, w) {
                (0, 0) => ("\u{2713}".to_string(), "sbadge clean"),
                (_, 0) => (format!("{e}E"), "sbadge err"),
                (0, _) => (format!("{w}W"), "sbadge warn"),
                _ => (format!("{e}E {w}W"), "sbadge err"),
            };
            let bw = (10 + 7 * label.chars().count()).max(22) as f64;
            let (x, tx) = (r0.w - bw - 6.0, r0.w - 6.0 - bw / 2.0);
            let tip = format!("{e} open error(s), {w} open warning(s)");
            view! {
                g(class=cls) {
                    svg_title { (tip) }
                    rect(x=x.to_string(), y="7", width=bw.to_string(), height="18", rx="9")
                    text(x=tx.to_string(), y="20", "text-anchor"="middle") { (label) }
                }
            }
        }
    };
    // Heat bar: length and intensity follow the worst value/limit ratio of the brick's functions.
    let heat = {
        let worst = brick.worst_ratio;
        let track = r0.w - 16.0;
        let fill = (track * heat_len(worst)).max(if worst > 0.0 { 2.0 } else { 0.0 });
        let cls = format!("heat h{}", heat_level(worst));
        let tip = if worst > 0.0 {
            format!("worst function is at {}x its limit", fmt_score(worst))
        } else {
            "no complexity limit applies".to_string()
        };
        view! {
            g(class=cls) {
                svg_title { (tip) }
                rect(class="track", x="8", y="29", width=track.to_string(), height="5", rx="2.5")
                rect(class="hbar", x="8", y="29", width=fill.to_string(), height="5", rx="2.5")
            }
        }
    };
    let (w, h) = (r0.w.to_string(), r0.h.to_string());
    // The box width follows the name up to a cap; a longer name is cut (the tooltip has it all).
    let fit = (((r0.w - 54.0) / 8.2) as usize).max(8);
    let name = if brick.name.chars().count() > fit {
        format!(
            "{}\u{2026}",
            brick.name.chars().take(fit - 1).collect::<String>()
        )
    } else {
        brick.name.clone()
    };
    let tag = brick.tag.clone();
    let (c1, c2) = (cx.clone(), cx.clone());
    let down = move |e: Click| {
        if e.button() != 0 {
            return;
        }
        e.stop_propagation();
        c1.moved.set(false);
        let d0 = c1.drag.with(|d| d.get(&b).copied().unwrap_or((0.0, 0.0)));
        c1.op.set(Some(Op::Drag {
            b,
            sx: e.client_x() as f64,
            sy: e.client_y() as f64,
            dx0: d0.0,
            dy0: d0.1,
        }));
    };
    let click = move |e: Click| {
        e.stop_propagation();
        if c2.moved.get() {
            c2.moved.set(false);
            return;
        }
        let sel = c2.eval.with(|ev| ev.sel.clone());
        let next = if e.shift_key() && matches!(sel, Sel::Brick(_) | Sel::Group { .. }) {
            sel.add_brick(b)
        } else {
            sel.click(b)
        };
        c2.select(next);
    };
    view! {
        g(class=cls, transform=transform, "data-id"=brick.name.clone(), on:mousedown=down, on:click=click) {
            svg_title { (tip) }
            rect(class="box", width=w, height=h, rx="6")
            text(x="8", y="22", "font-size"="15", "font-weight"="600") { (name) }
            (heat)
            text(x="8", y="48", class="sub") { (tag) }
            (badge)
        }
    }
}
