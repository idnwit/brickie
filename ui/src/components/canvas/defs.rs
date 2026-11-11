//! Arrow markers per edge kind plus the violating and accepted ones (`defsHTML`).

use sycamore::prelude::*;

/// `defsHTML`: arrow markers per kind plus the violating and accepted ones.
pub fn defs() -> View {
    let simple = |id: &'static str, color: &'static str, w: &'static str| {
        let style = format!("fill:{color}");
        view! {
            marker(id=id, "viewBox"="0 0 14 14", "refX"="13", "refY"="7", "markerWidth"=w, "markerHeight"=w, "markerUnits"="userSpaceOnUse", "orient"="auto") {
                path(d="M1,1.5 L13,7 L1,12.5 z", style=style)
            }
        }
    };
    let uses = simple("m-uses", "var(--k-uses)", "14");
    let viol = simple("m-viol", "var(--e-err)", "17");
    let acc = simple("m-acc", "var(--e-acc)", "16");
    let warn = simple("m-warn", "var(--e-warn)", "15");
    let n_uses = simple("m-n-uses", "var(--e-ok)", "14");
    view! {
        defs {
            (uses)
            marker(id="m-implements", "viewBox"="0 0 14 14", "refX"="13", "refY"="7", "markerWidth"="14", "markerHeight"="14", "markerUnits"="userSpaceOnUse", "orient"="auto") {
                path(d="M1.5,1.5 L13,7 L1.5,12.5 z", style="fill:var(--bg);stroke:var(--k-implements);stroke-width:1.8")
            }
            marker(id="m-reexports", "viewBox"="0 0 14 14", "refX"="13", "refY"="7", "markerWidth"="14", "markerHeight"="14", "markerUnits"="userSpaceOnUse", "orient"="auto") {
                path(d="M2,1.5 L12,7 L2,12.5", style="fill:none;stroke:var(--k-reexports);stroke-width:2")
            }
            marker(id="t-reexports", "viewBox"="0 0 16 14", "refX"="1", "refY"="7", "markerWidth"="16", "markerHeight"="14", "markerUnits"="userSpaceOnUse", "orient"="auto") {
                path(d="M1,7 L8,2 L15,7 L8,12 z", style="fill:var(--k-reexports)")
            }
            marker(id="m-n-implements", "viewBox"="0 0 14 14", "refX"="13", "refY"="7", "markerWidth"="14", "markerHeight"="14", "markerUnits"="userSpaceOnUse", "orient"="auto") {
                path(d="M1.5,1.5 L13,7 L1.5,12.5 z", style="fill:var(--bg);stroke:var(--e-ok);stroke-width:1.8")
            }
            marker(id="m-n-reexports", "viewBox"="0 0 14 14", "refX"="13", "refY"="7", "markerWidth"="14", "markerHeight"="14", "markerUnits"="userSpaceOnUse", "orient"="auto") {
                path(d="M2,1.5 L12,7 L2,12.5", style="fill:none;stroke:var(--e-ok);stroke-width:2")
            }
            marker(id="t-n-reexports", "viewBox"="0 0 16 14", "refX"="1", "refY"="7", "markerWidth"="16", "markerHeight"="14", "markerUnits"="userSpaceOnUse", "orient"="auto") {
                path(d="M1,7 L8,2 L15,7 L8,12 z", style="fill:var(--e-ok)")
            }
            (n_uses)
            (viol)
            (warn)
            (acc)
        }
    }
}
