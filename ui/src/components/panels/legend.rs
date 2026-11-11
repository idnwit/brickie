//! `renderLegend`: edge kinds (click to hide/show), edge states, brick colours.

use crate::components::dash;
use crate::state::*;
use sycamore::prelude::*;

/// One legend sample line (`renderLegend`'s `line`/`mk`), drawn by hand.
pub fn legend_line(
    kind: &str,
    color: &str,
    halo: bool,
    width: f64,
    op: f64,
    mark: Option<&str>,
) -> View {
    let c = color.to_string();
    let (x1, x2) = (if kind == "reexports" { 14.0 } else { 4.0 }, 52.0);
    let da = if mark == Some("\u{2713}") {
        "6 4"
    } else {
        dash(kind)
    };
    let line_style = format!("stroke:{c};stroke-width:{width};opacity:{op}");
    let cap = if kind == "reexports" { "round" } else { "butt" };
    let halo = if halo {
        let st = format!("stroke:{c};stroke-width:8;stroke-opacity:.2;stroke-linecap:round");
        view! { line(x1=x1.to_string(), y1="8", x2=x2.to_string(), y2="8", style=st) }
    } else {
        View::default()
    };
    let head = match kind {
        "implements" => {
            let st = format!("fill:var(--bg);stroke:{c};stroke-width:1.6");
            view! { path(d=format!("M{},3 L{},8 L{},13 z", x2 - 1.0, x2 + 4.0, x2 - 1.0), style=st) }
        }
        "reexports" => {
            let st = format!("fill:none;stroke:{c};stroke-width:1.8");
            let st2 = format!("fill:{c}");
            view! {
                path(d=format!("M{},3 L{},8 L{},13", x2 - 5.0, x2, x2 - 5.0), style=st)
                path(d="M1,8 L7,4 L13,8 L7,12 z", style=st2)
            }
        }
        _ => {
            let st = format!("fill:{c}");
            view! { path(d=format!("M{},3 L{},8 L{},13 z", x2 - 4.0, x2 + 3.0, x2 - 4.0), style=st) }
        }
    };
    let mark = match mark {
        Some(t) => {
            let st = format!("font-size:14px;font-weight:700;fill:{c}");
            let t = t.to_string();
            view! { text(x="30", y="14", "text-anchor"="middle", style=st) { (t) } }
        }
        None => View::default(),
    };
    view! {
        svg("viewBox"="0 0 56 16") {
            (halo)
            line(x1=x1.to_string(), y1="8", x2=x2.to_string(), y2="8", "stroke-dasharray"=da, "stroke-linecap"=cap, style=line_style)
            (head)
            (mark)
        }
    }
}

/// `renderLegend`: edge kinds (click to hide/show), edge states, brick colours.
pub fn legend(cx: &Cx) -> View {
    const MEANS: [(&str, &str); 3] = [
        (
            "uses",
            "A calls or names an item of B (the module that defines it, even via a re-export); if B changes, A may break",
        ),
        ("implements", "A implements a trait that lives in B"),
        (
            "reexports",
            "A re-exports an item of B (`pub use`, any pub visibility)",
        ),
    ];
    let kinds: Vec<View> = MEANS
        .iter()
        .map(|(k, m)| {
            let (c, key) = (cx.clone(), format!("k-{k}"));
            let key2 = key.clone();
            let cls = {
                let c = c.clone();
                move || if Lay::from_params(&c.params.get_clone()).on(&key) { "k" } else { "k off" }
            };
            let line = legend_line(k, &format!("var(--k-{k})"), false, 1.8, 1.0, None);
            view! {
                div(class=cls, "data-k"=k.to_string(), title=m.to_string(), on:click=move |_: Click| {
                    let p = with_toggle(&c.params.get_clone(), &key2);
                    c.go(p);
                }) { (line) span { b { (k.to_string()) } " " (m.to_string()) } }
            }
        })
        .collect();
    let kinds = View::from(kinds);
    let _ = MEANS;
    let states: Vec<View> = [
        (
            "allowed: light grey, thin (kind colours only in Architecture)",
            legend_line("uses", "var(--e-ok)", false, 1.2, 1.0, None),
        ),
        (
            "violating, error: dark red, thick, glow, mark \u{2717}",
            legend_line("uses", "var(--e-err)", true, 3.6, 1.0, Some("\u{2717}")),
        ),
        (
            "violating, warning: orange, medium",
            legend_line("uses", "var(--e-warn)", false, 2.6, 1.0, Some("\u{2717}")),
        ),
        (
            "accepted violation: red, dashed, faded, \u{2713}",
            legend_line("uses", "var(--e-acc)", false, 2.0, 0.55, Some("\u{2713}")),
        ),
        (
            "test-scope only (shown with the tests layer)",
            legend_line("uses", "var(--e-ok)", false, 1.2, 0.45, None),
        ),
    ]
    .into_iter()
    .map(|(t, l)| view! { div(class="k stat") { (l) span { (t) } } })
    .collect();
    let states = View::from(states);
    let bands: Vec<View> = [
        ("fill:var(--node);stroke:var(--b-ok);stroke-width:1.2", "correct: grey border, plain fill"),
        ("fill:var(--b-err-fill);stroke:var(--b-err);stroke-width:3", "open error: thick red border, light red fill"),
        ("fill:var(--b-warn-fill);stroke:var(--b-warn);stroke-width:2.4", "only open warnings: orange border and fill"),
        ("fill:var(--node);stroke:var(--b-err);stroke-width:1.6;stroke-dasharray:6 3", "all findings accepted: dashed border"),
    ]
    .into_iter()
    .map(|(st, t)| {
        view! { div(class="k stat") { svg("viewBox"="0 0 56 16") { rect(x="3", y="1", width="50", height="14", rx="3", style=st) } span { (t) } } }
    })
    .collect();
    let bands = View::from(bands);
    let scores = view! {
        div(class="k stat") { svg("viewBox"="0 0 56 16") { rect(x="2", y="1", width="26", height="14", rx="7", style="fill:var(--b-err)") rect(x="31", y="1", width="22", height="14", rx="7", style="fill:var(--ok)") } span { "corner badge: open errors/warnings (3E 1W), or a check when clean" } }
        div(class="k stat") { svg("viewBox"="0 0 56 16") { rect(x="2", y="5", width="52", height="6", rx="3", style="fill:var(--band)") rect(x="2", y="5", width="34", height="6", rx="3", style="fill:var(--h2)") } span { "heat bar: worst function value/limit (full bar = 2x the limit)" } }
        div(class="muted tiny") { "Score = errors + 0.25 x warnings + sum of (worst ratio - 1) over functions over a limit." }
    };
    view! {
        div(id="legend") {
            b { "Legend" } " " span(class="muted") { "(click a kind to hide/show)" }
            div(id="legbody") {
                h5 { "Edge kinds" }
                (kinds)
                h5 { "Edge states" }
                (states)
                h5 { "Bricks" }
                (bands)
                h5 { "Brick score" }
                (scores)
                div(class="muted tiny") { "Drag a brick to move it; drag the background to pan; wheel to zoom; Shift+drag to box-select; Shift+click adds to the selection." }
            }
        }
    }
}
