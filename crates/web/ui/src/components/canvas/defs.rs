//! Arrow markers per edge kind, the violating one, and draft/planned link markers (`defsHTML`).

use sycamore::prelude::*;

fn arrow_filled(id: String, color: &'static str, w: &'static str) -> View {
    let style = format!("fill:{color}");
    view! {
        marker(id=id, "viewBox"="0 0 14 14", "refX"="13", "refY"="7", "markerWidth"=w, "markerHeight"=w, "markerUnits"="userSpaceOnUse", "orient"="auto") {
            path(d="M1,1.5 L13,7 L1,12.5 z", style=style)
        }
    }
}

fn arrow_hollow(id: String, color: &'static str) -> View {
    let style = format!("fill:var(--bg);stroke:{color};stroke-width:1.2");
    view! {
        marker(id=id, "viewBox"="0 0 14 14", "refX"="13", "refY"="7", "markerWidth"="9", "markerHeight"="9", "markerUnits"="userSpaceOnUse", "orient"="auto") {
            path(d="M1.5,1.5 L13,7 L1.5,12.5 z", style=style)
        }
    }
}

fn arrow_open(id: String, color: &'static str) -> View {
    let style = format!("fill:none;stroke:{color};stroke-width:1.4");
    view! {
        marker(id=id, "viewBox"="0 0 14 14", "refX"="13", "refY"="7", "markerWidth"="9", "markerHeight"="9", "markerUnits"="userSpaceOnUse", "orient"="auto") {
            path(d="M2,1.5 L12,7 L2,12.5", style=style)
        }
    }
}

fn tail_diamond(id: String, color: &'static str) -> View {
    let style = format!("fill:{color}");
    view! {
        marker(id=id, "viewBox"="0 0 16 14", "refX"="1", "refY"="7", "markerWidth"="10", "markerHeight"="9", "markerUnits"="userSpaceOnUse", "orient"="auto") {
            path(d="M1,7 L8,2 L15,7 L8,12 z", style=style)
        }
    }
}

/// Markers for `[[link]]` lines: each kind in its own colour, plus override tones (`deny`, `agent`, `warn`).
fn link_markers() -> Vec<View> {
    let mut out = vec![
        arrow_filled("m-l-uses-uses".into(), "var(--l-uses)", "8"),
        arrow_hollow("m-l-implements-implements".into(), "var(--l-implements)"),
        arrow_open("m-l-reexports-reexports".into(), "var(--l-reexports)"),
        tail_diamond("t-l-reexports-reexports".into(), "var(--l-reexports)"),
    ];
    for (tone, color) in [
        ("deny", "var(--l-deny)"),
        ("agent", "var(--agent)"),
        ("warn", "var(--warn)"),
    ] {
        out.push(arrow_filled(format!("m-l-uses-{tone}"), color, "8"));
        out.push(arrow_hollow(format!("m-l-implements-{tone}"), color));
        out.push(arrow_open(format!("m-l-reexports-{tone}"), color));
        out.push(tail_diamond(format!("t-l-reexports-{tone}"), color));
    }
    out
}

/// `defsHTML`: arrow markers per kind plus the violating and link markers.
pub fn defs() -> View {
    let uses = arrow_filled("m-uses".into(), "var(--k-uses)", "9");
    let impls = arrow_hollow("m-implements".into(), "var(--k-implements)");
    let reexp = arrow_open("m-reexports".into(), "var(--k-reexports)");
    let t_reexp = tail_diamond("t-reexports".into(), "var(--k-reexports)");
    let n_uses = arrow_filled("m-n-uses".into(), "var(--e-ok)", "9");
    let n_impls = arrow_hollow("m-n-implements".into(), "var(--e-ok)");
    let n_reexp = arrow_open("m-n-reexports".into(), "var(--e-ok)");
    let t_n_reexp = tail_diamond("t-n-reexports".into(), "var(--e-ok)");
    let viol = arrow_filled("m-viol".into(), "var(--e-err)", "10");
    let links = View::from(link_markers());
    view! {
        defs {
            (uses)
            (impls)
            (reexp)
            (t_reexp)
            (n_impls)
            (n_reexp)
            (t_n_reexp)
            (n_uses)
            (viol)
            (links)
        }
    }
}
