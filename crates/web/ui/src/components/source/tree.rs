//! The vertical complexity tree (`vertHTML`, `iceCol`, `heat` of the original): a gutter with one
//! column per nesting depth and one bar per decision branch over the lines it spans.

use brickie_model::{FnTree, TreeNode};
use sycamore::prelude::*;

/// Height of one code line and width of one tree column, in pixels (`LH`, `CW`).
pub const LH: u32 = 18;
pub const CW: u32 = 14;

/// `iceCol`: cold to hot with nesting depth.
pub fn ice_col(depth: u32, max: u32) -> String {
    let t = (f64::from(depth) / f64::from(max.max(1))).min(1.0);
    format!("hsl({} 68% 62%)", (215.0 - 215.0 * t).round())
}

fn log_share(paths: u64, root: u64) -> f64 {
    (paths.max(1) as f64).ln() / (root.max(2) as f64).ln()
}

/// Background of a code line: the deepest tree node covering it, `heat` of the original.
pub fn heat(t: &FnTree, max: u32, line: u32) -> Option<String> {
    let n = t
        .nodes
        .iter()
        .filter(|n| n.line <= line && line <= n.end_line)
        .max_by_key(|n| n.depth)?;
    let hue = (215.0 - 215.0 * (f64::from(n.depth) / f64::from(max.max(1))).min(1.0)).round();
    Some(format!(
        "background:hsla({hue},68%,55%,{:.3})",
        0.05 + 0.2 * log_share(n.paths, t.paths)
    ))
}

pub fn bar(n: &TreeNode, t: &FnTree, max: u32) -> View {
    let top = (n.line - 1) * LH;
    let h = (n.end_line - n.line + 1) * LH - 1;
    let op = 0.35 + 0.65 * log_share(n.paths, t.paths);
    let style = format!(
        "left:{}px;top:{top}px;width:{}px;height:{h}px;background:{};opacity:{op:.2}",
        n.depth * CW,
        CW - 1,
        ice_col(n.depth, max)
    );
    let tip = format!(
        "{}: {}\nlines {}-{} \u{00B7} paths {} \u{00B7} depth {}",
        n.kind, n.label, n.line, n.end_line, n.paths, n.depth
    );
    let label = if h >= 11 && n.paths > 1 {
        n.paths.to_string()
    } else {
        String::new()
    };
    let (a, b) = (n.line.to_string(), n.end_line.to_string());
    view! { div(class="vb", style=style, title=tip, "data-a"=a, "data-b"=b) { (label) } }
}

pub fn tree_summary(t: &FnTree) -> View {
    let text = format!(
        "{} \u{00B7} paths {} \u{00B7} cognitive {} \u{00B7} cyclomatic {} \u{00B7} nesting {} \u{00B7} {} lines",
        t.label, t.paths, t.cognitive, t.cyclomatic, t.max_depth, t.lines
    );
    view! { div(class="tsum") { (text) } }
}
