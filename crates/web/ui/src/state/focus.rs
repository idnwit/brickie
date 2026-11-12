//! What a selected finding or selection shows on the canvas (`focusSpec`, `fSpec`).

use crate::model::*;
use crate::state::Sel;
use std::collections::BTreeSet;

/// What a selected finding or selection shows on the canvas (`focusSpec`).
#[derive(Debug, Clone, Default)]
pub struct Focus {
    pub bricks: BTreeSet<usize>,
    /// Edge indices that stay visible.
    pub keep: BTreeSet<usize>,
    /// Edges drawn emphasised.
    pub hot: BTreeSet<usize>,
    /// A group selection (edge labels stay hidden).
    pub group: bool,
    /// The focus comes from a selected finding.
    pub finding: bool,
    /// Bricks drawn at full strength (the rest is dimmed): the focus plus, for a brick or
    /// edge selection, the other ends of the kept edges.
    pub lit: BTreeSet<usize>,
}

pub fn brick_ix(g: &GraphView, name: &str) -> Option<usize> {
    g.bricks.iter().position(|b| b.name == name)
}

fn is_dep(code: &str) -> bool {
    code.starts_with("deps/")
}

/// `fSpec`: the focus of a selected finding.
pub fn finding_focus(g: &GraphView, d: &DiagView) -> Focus {
    let set: BTreeSet<usize> = diag_bricks(g, d).into_iter().collect();
    let touching = |e: &EdgeView| set.contains(&e.from) || set.contains(&e.to);
    let (keep, hot): (BTreeSet<usize>, BTreeSet<usize>) = if is_dep(&d.code) {
        let k: BTreeSet<usize> = (0..g.edges.len())
            .filter(|&i| g.edges[i].diags.contains(&d.id))
            .collect();
        (k.clone(), k)
    } else if set.len() > 1 {
        let k: BTreeSet<usize> = (0..g.edges.len())
            .filter(|&i| set.contains(&g.edges[i].from) && set.contains(&g.edges[i].to))
            .collect();
        (k.clone(), k)
    } else {
        (
            (0..g.edges.len())
                .filter(|&i| touching(&g.edges[i]))
                .collect(),
            BTreeSet::new(),
        )
    };
    Focus {
        lit: set.clone(),
        bricks: set,
        keep,
        hot,
        group: false,
        finding: true,
    }
}

fn with_lit(g: &GraphView, mut f: Focus) -> Focus {
    f.lit = f.bricks.clone();
    for &e in &f.keep {
        f.lit.insert(g.edges[e].from);
        f.lit.insert(g.edges[e].to);
    }
    f
}

/// `focusSpec`: a selected finding wins over the brick/edge/group selection.
pub fn focus_of(g: &GraphView, sel: &Sel, p: &Params, diags: &[DiagView]) -> Option<Focus> {
    if let Some(d) = diags.iter().find(|d| !p.item.is_empty() && d.id == p.item) {
        return Some(finding_focus(g, d));
    }
    let touching = |set: &BTreeSet<usize>| -> BTreeSet<usize> {
        (0..g.edges.len())
            .filter(|&i| set.contains(&g.edges[i].from) || set.contains(&g.edges[i].to))
            .collect()
    };
    match sel {
        Sel::None => None,
        Sel::Brick(b) => {
            let set: BTreeSet<usize> = [*b].into();
            let k = touching(&set);
            Some(with_lit(
                g,
                Focus {
                    lit: BTreeSet::new(),
                    bricks: set,
                    keep: k.clone(),
                    hot: k,
                    group: false,
                    finding: false,
                },
            ))
        }
        Sel::Group { .. } => {
            let set = sel.set(g);
            Some(Focus {
                keep: touching(&set),
                lit: set.clone(),
                bricks: set,
                hot: BTreeSet::new(),
                group: true,
                finding: false,
            })
        }
        Sel::Edge(e) => Some(with_lit(
            g,
            Focus {
                lit: BTreeSet::new(),
                bricks: [g.edges[*e].from, g.edges[*e].to].into(),
                keep: [*e].into(),
                hot: BTreeSet::new(),
                group: false,
                finding: false,
            },
        )),
    }
}
