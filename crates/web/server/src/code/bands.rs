//! Complexity as bands: for every measured function of a file, one band over the function
//! (depth 0, coloured by its worst metric against its limit) and one per node of its decision
//! tree (computed on demand with tree-sitter, coloured by nesting depth against the nesting
//! limit).

use super::read::read_confined;
use brickie_check::check::{Loaded, complexity_limits};
use brickie_facts::Function;
use brickie_model::{FnTree, TreeNode};
use codeview_port::Band;
use std::path::Path;

/// The bands of `path`, outer bands first.
pub fn bands(l: &Loaded, path: &str) -> Vec<Band> {
    let Some(fd) = l.cache.files.get(path) else {
        return Vec::new();
    };
    let Some(bytes) = read_confined(Path::new(&l.cache.workspace_root), path) else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut out = Vec::new();
    for func in &fd.functions {
        let Some(tree) = brickie_measure::fn_tree(&text, func.line) else {
            continue;
        };
        let leaf = func.module.as_deref().and_then(|m| l.resolved.path(m));
        let limits = leaf.map_or_else(Vec::new, |leaf| complexity_limits(&l.cfg, leaf, func));
        out.extend(function_bands(func, &tree, &limits));
    }
    out.sort_by_key(|b| (b.start_line, b.depth));
    out
}

/// The function's band and its tree's; `limits` are `(metric, value, limit)`.
pub fn function_bands(func: &Function, t: &FnTree, limits: &[(&str, u64, u64)]) -> Vec<Band> {
    let worst = limits
        .iter()
        .map(|(_, v, l)| *v as f32 / (*l).max(1) as f32)
        .fold(0.0f32, f32::max);
    let over: Vec<String> = limits
        .iter()
        .map(|(m, v, l)| {
            let mark = if v > l { " (over)" } else { "" };
            format!("{m} {v} / limit {l}{mark}")
        })
        .collect();
    let mut detail = format!(
        "cognitive {} \u{b7} cyclomatic {} \u{b7} nesting {} \u{b7} paths {} \u{b7} {} lines",
        t.cognitive, t.cyclomatic, t.max_depth, t.paths, t.lines
    );
    if !over.is_empty() {
        detail += &format!("\n{}", over.join("\n"));
    }
    // Nesting is coloured against its limit when there is one (else the function's deepest).
    let deepest = t.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let nesting = limits.iter().find(|(m, _, _)| *m == "nesting");
    let max = nesting.map_or(deepest, |(_, _, l)| deepest.max(*l as u32));
    let mut out = vec![Band {
        start_line: func.line,
        end_line: func.end_line.max(func.line),
        depth: 0,
        heat: worst.min(1.0),
        label: format!("fn {}", t.label),
        detail,
    }];
    out.extend(t.nodes.iter().map(|n| node_band(n, t, max)));
    out
}

fn node_band(n: &TreeNode, t: &FnTree, max: u32) -> Band {
    Band {
        start_line: n.line,
        end_line: n.end_line.max(n.line),
        depth: n.depth,
        heat: (n.depth as f32 / max.max(1) as f32).min(1.0),
        label: format!("{} {} \u{2014} nesting {}", n.kind, n.label, n.depth),
        detail: format!("paths {} of {} in fn {}", n.paths, t.paths, t.label),
    }
}
