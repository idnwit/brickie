//! The source popup page: a file with line numbers, the target line or span highlighted, and the
//! vertical complexity tree of the function around the target line in a gutter at the left
//! (`vertHTML` of the original). Rendered on the server only (it is loaded in an iframe and needs
//! no client state).

mod popup;
mod tree;

use brickie_web_api::SourcePage;
use sycamore::prelude::*;
use tree::{bar, heat, tree_summary};

pub use popup::modal_view;
pub use tree::{CW, LH, ice_col};

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn source_view(page: SourcePage) -> View {
    let file = page.file;
    let tree = page.tree;
    let nlines = page.lines.len() as u32;
    let max = tree
        .as_ref()
        .map_or(0, |t| t.nodes.iter().map(|n| n.depth).max().unwrap_or(0));
    let heats: Vec<Option<String>> = page
        .lines
        .iter()
        .map(|l| tree.as_ref().and_then(|t| heat(t, max, l.n)))
        .collect();
    let rows: Vec<View> = page
        .lines
        .into_iter()
        .zip(heats)
        .map(|(l, heat)| {
            let id = format!("L{}", l.n);
            let href = format!("#L{}", l.n);
            let class = if l.line_hl { "cl hl" } else { "cl" };
            let (n, n2) = (l.n.to_string(), l.n.to_string());
            let (before, hit, after) = (l.before, l.hit, l.after);
            let style = heat.unwrap_or_default();
            view! {
                div(id=id, class=class, style=style, "data-n"=n2) {
                    a(class="n", href=href) { (n) }
                    span(class="t") { (before) mark { (hit) } (after) }
                }
            }
        })
        .collect();
    let rows = View::from(rows);
    let (summary, gutter) = match &tree {
        Some(t) => {
            let bars: Vec<View> = t.nodes.iter().map(|n| bar(n, t, max)).collect();
            let gh = format!("width:{}px;height:{}px", (max + 1) * CW, nlines * LH);
            let bars = View::from(bars);
            let w = format!("width:{}px", (max + 1) * CW);
            (
                tree_summary(t),
                view! { div(class="gut", style=w) { div(class="gin", style=gh) { (bars) } } },
            )
        }
        None => (View::default(), View::default()),
    };
    view! {
        h1(class="file") { (file) }
        (summary)
        div(class="tvb") { div(class="tvi") { (gutter) div(class="cd") { (rows) } } }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use brickie_model::{FnTree, TreeNode};
    use brickie_web_api::SourceLine;

    #[test]
    fn tree_gutter_and_heat_render_beside_the_code() {
        let node = |kind: &str, line, end_line, paths, depth| TreeNode {
            kind: kind.into(),
            label: "x > 0".into(),
            line,
            end_line,
            paths,
            depth,
        };
        let line = |n| SourceLine {
            n,
            before: String::new(),
            hit: String::new(),
            after: format!("l{n}"),
            line_hl: n == 2,
        };
        let page = SourcePage {
            file: "a.rs".into(),
            lines: (1..=4).map(line).collect(),
            tree: Some(FnTree {
                label: "f".into(),
                line: 1,
                end_line: 4,
                lines: 4,
                paths: 4,
                cognitive: 3,
                cyclomatic: 3,
                max_depth: 2,
                nodes: vec![node("if", 2, 4, 4, 1), node("loop", 3, 3, 2, 2)],
            }),
        };
        let html = crate::render_source(&page);
        assert_eq!(html.matches("class=\"vb\"").count(), 2, "{html}");
        // A bar sits at its first line (18 px per line), one column per depth.
        assert!(html.contains("left:14px;top:18px"), "{html}");
        assert!(html.contains("f \u{b7} paths 4"), "{html}");
        assert!(html.contains("background:hsla("));
        assert_eq!(ice_col(0, 3), "hsl(215 68% 62%)");
        assert_eq!(ice_col(9, 3), "hsl(0 68% 62%)");
    }
}
