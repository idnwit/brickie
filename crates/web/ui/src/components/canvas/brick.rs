//! One brick: box, name, heat bar, tag line and the score badge (`renderGraph` node part). A
//! composite has a double border, its exported children as ports on the bottom border and
//! zooms in on double-click; a faded outside neighbour goes to its own level.

use crate::layout::Rect;
use crate::state::*;
use brickie_web_api::{BrickView, fmt_score, heat_len, heat_level};
use sycamore::prelude::*;

/// One brick: box, name, heat bar, tag line and the score badge (`renderGraph` node part).
pub fn node_view(cx: &Cx, b: usize, pos: ReadSignal<Vec<Rect>>) -> View {
    let brick = cx.m.g.bricks[b].clone();
    let r0 = cx.m.layout.pos[b];
    let tip = {
        let (cx, name, tag) = (cx.clone(), brick.name.clone(), brick.tag.clone());
        let desc = brick
            .desc
            .as_ref()
            .map_or(String::new(), |d| format!("\n{d}"));
        let peek = peek(&brick);
        move || {
            let s = cx.scores.with(|s| s[b].clone());
            format!(
                "{name} [{tag}]{desc}\nscore {} ({}E {}W, complexity excess {}){peek}",
                fmt_score(s.score),
                s.errors,
                s.warns,
                fmt_score(s.excess)
            )
        }
    };
    let kind = kind_class(&brick);
    let cls = {
        let cx = cx.clone();
        move || {
            let mut c = format!("node{kind}");
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
    let shown = brick
        .name
        .strip_prefix("root/")
        .unwrap_or(&brick.name)
        .to_string();
    let name = if shown.chars().count() > fit {
        format!(
            "{}\u{2026}",
            shown.chars().take(fit - 1).collect::<String>()
        )
    } else {
        shown
    };
    let tag = if brick.composite() {
        format!("{} \u{00B7} {} bricks", brick.tag, brick.children.len())
    } else {
        brick.tag.clone()
    };
    let (c1, c2) = (cx.clone(), cx.clone());
    let down = move |e: Click| {
        if !grabs(&c1, &e) {
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
    // A second click on the same brick within `DOUBLE_MS` zooms in. The browser's own
    // `dblclick` never fires here: the first click re-renders the brick under the pointer.
    let target = zoom_target(&brick);
    let click = move |e: Click| {
        e.stop_propagation();
        if c2.moved.get() {
            c2.moved.set(false);
            return;
        }
        let now = e.time_stamp();
        let double = LAST_CLICK.with(|l| {
            let hit = matches!(l.get(), Some((lb, t)) if lb == b && now - t < DOUBLE_MS);
            l.set(if hit { None } else { Some((b, now)) });
            hit
        });
        if double {
            if let Some(t) = &target {
                c2.go(c2.params.with(|p| p.at_level(t)));
            }
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
    let inner = if brick.composite() {
        let (iw, ih) = ((r0.w - 8.0).to_string(), (r0.h - 8.0).to_string());
        view! { rect(class="box2", x="4", y="4", width=iw, height=ih, rx="4") }
    } else {
        View::default()
    };
    let ports = ports(&brick, r0.w, r0.h);
    let zoom = super::zoom_icon::zoom_icon(cx, &brick);
    let tag_x = if brick.composite() && !brick.ghost {
        8.0 + super::zoom_icon::ZOOM_ICON_W
    } else {
        8.0
    };
    view! {
        g(class=cls, transform=transform, "data-id"=brick.name.clone(), tabindex="0",
            on:mousedown=down, on:click=click) {
            svg_title { (tip) }
            rect(class="box", width=w, height=h, rx="6")
            (inner)
            (ports)
            text(x="8", y="22", "font-size"="15", "font-weight"="600") { (name) }
            (heat)
            text(x=tag_x.to_string(), y="48", class="sub") { (tag) }
            (badge)
            (zoom)
        }
    }
}

/// The box classes of a brick's kind, with the change preview marks: planned (no code yet)
/// and agent-proposed bricks.
fn kind_class(b: &BrickView) -> String {
    let kind = match (b.ghost, b.composite()) {
        (true, _) => " ghost",
        (_, true) => " comp",
        _ => "",
    };
    let planned = if b.planned { " planned" } else { "" };
    let proposed = if b.proposed { " proposed" } else { "" };
    format!("{kind}{planned}{proposed}")
}

/// A press that grabs the brick: left button, not the Hand tool (which pans from anywhere).
fn grabs(cx: &Cx, e: &Click) -> bool {
    e.button() == 0 && cx.ed.tool.get() != Tool::Hand
}

/// Two clicks on one brick closer than this (ms) are a double-click.
const DOUBLE_MS: f64 = 450.0;

thread_local! {
    /// The last brick click: brick index and event time.
    static LAST_CLICK: std::cell::Cell<Option<(usize, f64)>> = const { std::cell::Cell::new(None) };
}

/// The level a double-click opens: a composite's own level, a neighbour's enclosing level.
fn zoom_target(b: &BrickView) -> Option<String> {
    if b.ghost {
        b.path.rsplit_once('/').map(|(up, _)| up.to_string())
    } else if b.composite() {
        Some(b.path.clone())
    } else {
        None
    }
}

/// The hover peek: a composite's children (exported ones marked), or where a neighbour lives.
fn peek(b: &BrickView) -> String {
    if b.ghost {
        return format!(
            "\noutside this level: {}\ndouble-click to open its level",
            b.path
        );
    }
    if !b.composite() {
        return String::new();
    }
    let kids: Vec<String> = b
        .children
        .iter()
        .map(|k| {
            let mark = if b.exports.contains(k) {
                "\u{25CF}"
            } else {
                " "
            };
            format!("  {mark} {k}")
        })
        .collect();
    format!(
        "\ncomposite ({}), \u{25CF} exported:\n{}\ndouble-click to zoom in",
        b.file.as_deref().unwrap_or(""),
        kids.join("\n")
    )
}

/// Exported children of a composite as ports on its bottom border; a brick exported by the
/// current level gets a port on its top border.
fn ports(b: &BrickView, w: f64, h: f64) -> View {
    let mut out: Vec<View> = Vec::new();
    if b.exported {
        let x = (w / 2.0).to_string();
        out.push(view! { circle(class="port", cx=x, cy="0", r="4") { svg_title { "exported by this level" } } });
    }
    let fit = ((w - 12.0) / 56.0).max(1.0) as usize;
    for (i, name) in b.exports.iter().take(fit).enumerate() {
        let x = 14.0 + i as f64 * 56.0;
        let (cx, tx, y, ty) = (
            x.to_string(),
            (x + 7.0).to_string(),
            h.to_string(),
            (h + 13.0).to_string(),
        );
        let label: String = name.chars().take(7).collect();
        let tip = format!("port: {name} (exported)");
        out.push(view! {
            g(class="port") {
                svg_title { (tip) }
                circle(cx=cx, cy=y, r="4")
                text(x=tx, y=ty) { (label) }
            }
        });
    }
    View::from(out)
}
