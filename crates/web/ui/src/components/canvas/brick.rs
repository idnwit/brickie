//! One brick: box, name, heat bar, tag line and the score badge (`renderGraph` node part). A
//! composite has a double border, its exported children as ports on the bottom border and
//! zooms in on double-click; a faded outside neighbour goes to its own level.

use super::style::{Badge, Heat, NodeFlags, fit_name};
use crate::layout::Rect;
use crate::state::*;
use brickie_web_api::{BrickScore, BrickView, INDEPENDENT_TIP, fmt_score};
use sycamore::prelude::*;

/// One brick: box, name, heat bar, tag line and the score badge (`renderGraph` node part).
pub fn node_view(cx: &Cx, b: usize, pos: ReadSignal<Vec<Rect>>) -> View {
    let brick = cx.m.g.bricks[b].clone();
    let r0 = cx.m.layout.pos[b];
    let tip = {
        let cx = cx.clone();
        let brick = brick.clone();
        move || brick_tip(&brick, &cx.scores.with(|s| s[b].clone()))
    };
    let kind = kind_class(&brick);
    let cls = {
        let cx = cx.clone();
        move || node_flags(&cx, b).class(&kind)
    };
    let transform = move || pos.with(|p| format!("translate({},{})", p[b].x, p[b].y));
    let badge = {
        let cx = cx.clone();
        move || {
            let e = cx.scores.with(|s| s[b].errors);
            let bd = Badge::new(e, r0.w);
            let tip = format!("{e} finding(s)");
            view! {
                g(class=bd.cls) {
                    svg_title { (tip) }
                    rect(x=bd.x.to_string(), y="7", width=bd.bw.to_string(), height="18", rx="9")
                    text(x=bd.tx.to_string(), y="20", "text-anchor"="middle") { (bd.label) }
                }
            }
        }
    };
    let heat = heat_view(brick.worst_ratio, r0.w);
    let (w, h) = (r0.w.to_string(), r0.h.to_string());
    let name = name_view(&brick, r0.w);
    let tag = tag_line(&brick);
    let (c1, c2) = (cx.clone(), cx.clone());
    let down = move |e: Click| press(&c1, b, &e);
    let target = zoom_target(&brick);
    let click = move |e: Click| click_brick(&c2, b, target.as_deref(), &e);
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
            (name)
            (heat)
            text(x=tag_x.to_string(), y="48", class="sub") { (tag) }
            (badge)
            (zoom)
        }
    }
}

/// The heat bar of the worst function.
fn heat_view(worst: f64, w: f64) -> View {
    let h = Heat::new(worst, w);
    view! {
        g(class=h.cls) {
            svg_title { (h.tip) }
            rect(class="track", x="8", y="29", width=h.track.to_string(), height="5", rx="2.5")
            rect(class="hbar", x="8", y="29", width=h.fill.to_string(), height="5", rx="2.5")
        }
    }
}

/// The name, after a small book on an independent lib (a design flag). The
/// box width follows the name up to a cap; a longer name is cut (the tooltip has it all).
fn name_view(b: &BrickView, w: f64) -> View {
    let (book, x, room) = match b.independent {
        true => (book_icon(), "24", w - 16.0),
        false => (View::default(), "8", w),
    };
    let name = fit_name(&b.name, room);
    view! { (book) text(x=x, y="22", "font-size"="15", "font-weight"="600") { (name) } }
}

/// The independent lib mark: an open book (12 x 10) left of the name, titled with the tip.
fn book_icon() -> View {
    view! {
        g(class="indep-ico", transform="translate(8,11)") {
            svg_title { (INDEPENDENT_TIP) }
            path(d="M6 2C4.5 0.8 2.5 0.5 0.5 0.8V9.3C2.5 9 4.5 9.3 6 10.5C7.5 9.3 9.5 9 11.5 9.3V0.8C9.5 0.5 7.5 0.8 6 2ZM6 2V10.5")
        }
    }
}

/// The sub line: tag and child count of a composite.
fn tag_line(b: &BrickView) -> String {
    let mut tag = b.tag.clone();
    if b.composite() {
        tag = format!("{tag} \u{00B7} {} bricks", b.children.len());
    }
    tag
}

/// The hover text: name, tag, description, score and the peek.
fn brick_tip(b: &BrickView, s: &BrickScore) -> String {
    let desc = b.desc.as_ref().map_or(String::new(), |d| format!("\n{d}"));
    let indep = if b.independent {
        format!("\n{INDEPENDENT_TIP}")
    } else {
        String::new()
    };
    format!(
        "{} [{}]{desc}{indep}\nscore {} ({} finding(s), complexity excess {}){}",
        b.name,
        b.tag,
        fmt_score(s.score),
        s.errors,
        fmt_score(s.excess),
        peek(b)
    )
}

/// What the evaluation says about brick `b`, for its state classes.
fn node_flags(cx: &Cx, b: usize) -> NodeFlags {
    let (viol, item) = cx
        .params
        .with(|p| (p.preset() == "viol", !p.item.is_empty()));
    let hov = cx
        .hover
        .get()
        .is_some_and(|h| finding_focus(&cx.m.g, &cx.m.diags[h]).bricks.contains(&b));
    cx.eval.with(|ev| {
        let open = ev.node[b];
        let f = ev.focus.as_ref();
        NodeFlags {
            off_focus: f.is_some_and(|f| !f.lit.contains(&b)),
            viol,
            open,
            selected: if item {
                f.is_some_and(|f| f.finding && f.bricks.contains(&b))
            } else {
                ev.sel == Sel::Brick(b)
            },
            hov,
        }
    })
}

/// The box classes of a brick's kind.
fn kind_class(b: &BrickView) -> String {
    let kind = match (b.ghost, b.composite()) {
        (true, _) => " ghost",
        (_, true) => " comp",
        _ => "",
    };
    let independent = if b.independent { " independent" } else { "" };
    let unassigned = if b.unassigned { " unassigned" } else { "" };
    format!("{kind}{independent}{unassigned}")
}

/// A press on brick `b` starts a drag when it grabs the brick.
fn press(cx: &Cx, b: usize, e: &Click) {
    if !grabs(cx, e) {
        return;
    }
    e.stop_propagation();
    cx.moved.set(false);
    let d0 = cx.drag.with(|d| d.get(&b).copied().unwrap_or((0.0, 0.0)));
    cx.op.set(Some(Op::Drag {
        b,
        sx: e.client_x() as f64,
        sy: e.client_y() as f64,
        dx0: d0.0,
        dy0: d0.1,
    }));
}

/// A second click on the same brick within `DOUBLE_MS` zooms in to `target`. The browser's own
/// `dblclick` never fires here: the first click re-renders the brick under the pointer.
fn click_brick(cx: &Cx, b: usize, target: Option<&str>, e: &Click) {
    e.stop_propagation();
    if cx.moved.get() {
        cx.moved.set(false);
        return;
    }
    let now = e.time_stamp();
    let double = LAST_CLICK.with(|l| {
        let hit = matches!(l.get(), Some((lb, t)) if lb == b && now - t < DOUBLE_MS);
        l.set(if hit { None } else { Some((b, now)) });
        hit
    });
    if double {
        if let Some(t) = target {
            cx.go(cx.params.with(|p| p.at_level(t)));
        }
        return;
    }
    let sel = cx.eval.with(|ev| ev.sel.clone());
    let next = if e.shift_key() && matches!(sel, Sel::Brick(_) | Sel::Group { .. }) {
        sel.add_brick(b)
    } else {
        sel.click(b)
    };
    cx.select(next);
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
