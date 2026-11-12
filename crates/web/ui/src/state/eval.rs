//! What the page shows for the current params: which findings pass the layers, which edges are
//! drawn and how, the per-brick finding counts and the selection focus. Pure; the app wraps it in
//! a memo. Ports `fOn`/`vOn`, `violIds`, `edgeOn`, `fSpec` and `focusSpec`.

use super::*;
use brickie_web_api::*;
use std::collections::BTreeSet;

/// Effective status of a drawn edge (`edgeOn`): `ok`, `debt` (violating) or `accepted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EStat {
    Ok,
    Debt,
    Accepted,
}

#[derive(Debug, Clone)]
pub struct Eval {
    pub sel: Sel,
    /// `vOn` per diagnostic.
    pub von: Vec<bool>,
    /// Drawn status per edge, `None` when no enabled layer shows it.
    pub edge: Vec<Option<EStat>>,
    /// Per brick: open findings counted by `vOn`, and whether one is an error.
    pub node: Vec<(usize, bool)>,
    /// Per brick: it has shown findings and all of them are accepted.
    pub node_acc: Vec<bool>,
    pub focus: Option<Focus>,
}

pub fn evaluate(g: &GraphView, diags: &[DiagView], accepted: &[bool], p: &Params) -> Eval {
    let lay = Lay::from_params(p);
    let sel = Sel::from_params(p, g);
    let von: Vec<bool> = diags
        .iter()
        .map(|d| v_on(&lay, p, &d.code, &d.level))
        .collect();
    let by_id = |id: &str| diags.iter().position(|d| d.id == id);
    let viol_preset = p.preset() == "viol";

    // `violIds`: edges a deps finding points at (violating edges, cycle members).
    let mut viol: BTreeSet<usize> = BTreeSet::new();
    if viol_preset && matches!(p.vscope(), "all" | "deps") {
        for (i, e) in g.edges.iter().enumerate() {
            if e.diags.iter().filter_map(|id| by_id(id)).any(|c| von[c]) {
                viol.insert(i);
            }
        }
    }

    let edge: Vec<Option<EStat>> = g
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| {
            // One of the two boundary drawings: to the children or aggregated at the frame.
            let hidden = if p.to_frame() {
                is_boundary(g, e)
            } else {
                e.frame()
            };
            if hidden || !lay.on(&format!("k-{}", e.kind)) {
                return None;
            }
            let cards: Vec<usize> = e.diags.iter().filter_map(|id| by_id(id)).collect();
            let dbt = !cards.is_empty();
            let dbt_on = cards.iter().any(|&c| f_on(&lay, &diags[c].code));
            let all_accepted = !cards.is_empty() && cards.iter().all(|&c| accepted[c]);
            if viol_preset {
                if !viol.contains(&i) {
                    return None;
                }
                return Some(if all_accepted {
                    EStat::Accepted
                } else {
                    EStat::Debt
                });
            }
            if e.test && !lay.on("tests") && !(dbt && dbt_on) {
                return None;
            }
            if !(lay.on("edges") || (dbt && dbt_on)) {
                return None;
            }
            Some(if dbt && dbt_on {
                if all_accepted {
                    EStat::Accepted
                } else {
                    EStat::Debt
                }
            } else {
                EStat::Ok
            })
        })
        .collect();

    let mut node = vec![(0usize, false); g.bricks.len()];
    let mut node_acc = vec![false; g.bricks.len()];
    for (c, d) in diags.iter().enumerate() {
        if !von[c] {
            continue;
        }
        if accepted[c] {
            for b in diag_bricks(g, d) {
                node_acc[b] = true;
            }
            continue;
        }
        for b in diag_bricks(g, d) {
            node[b].0 += 1;
            node[b].1 |= d.level == "error";
        }
    }
    let focus = focus_of(g, &sel, p, diags);
    Eval {
        sel,
        von,
        edge,
        node,
        node_acc,
        focus,
    }
}

/// `listFindings` (without the text filter): the diagnostics shown in the left list.
pub fn list_findings(
    g: &GraphView,
    diags: &[DiagView],
    accepted: &[bool],
    p: &Params,
    ev: &Eval,
) -> Vec<usize> {
    let scope = ev.sel.scope(g);
    (0..diags.len())
        .filter(|&i| {
            let d = &diags[i];
            ev.von[i]
                && scope
                    .as_ref()
                    .is_none_or(|s| diag_bricks(g, d).iter().any(|b| s.contains(b)))
                && (p.below.is_empty()
                    || under(d.brick_level.as_deref().unwrap_or("root"), p.level_path()))
                && (p.grp.is_empty() || group_sub(&d.code).0 == p.grp)
                && code_matches(&d.code, &p.code)
                && (p.level.is_empty() || d.level == p.level)
                && match p.status.as_str() {
                    "open" => !accepted[i],
                    "accepted" => accepted[i],
                    _ => true,
                }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(id: &str, code: &str, level: &str, bricks: &[&str]) -> DiagView {
        DiagView {
            id: id.into(),
            level: level.into(),
            code: code.into(),
            message: String::new(),
            span: None,
            spans: vec![],
            rule: None,
            help: None,
            notes: vec![],
            bricks: bricks.iter().map(|s| s.to_string()).collect(),
            accepted: false,
            accepted_reason: None,
            snippet: None,
            brick_level: None,
        }
    }

    fn graph() -> GraphView {
        let b = |n: &str, row| BrickView {
            name: n.into(),
            tag: "t".into(),
            row,
            tags: vec!["t".into()],
            members: vec![],
            worst_ratio: 0.0,
            hot: vec![],
            ..Default::default()
        };
        let e = |from, to, diags: &[&str]| EdgeView {
            from,
            to,
            kind: "uses".into(),
            test: false,
            diags: diags.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        GraphView {
            rows: vec!["t".into()],
            bricks: vec![b("a", 0), b("b", 0), b("c", 0)],
            edges: vec![e(0, 1, &["d1"]), e(1, 2, &[]), e(2, 0, &[])],
        }
    }

    #[test]
    fn architecture_shows_all_edges_with_violating_ones_marked() {
        let g = graph();
        let diags = vec![diag("d1", "deps/deny", "error", &["a", "b"])];
        let arch = evaluate(&g, &diags, &[false], &Params::default());
        // Architecture draws every edge and marks the violating one; complexity and stale
        // findings stay hidden.
        assert_eq!(
            arch.edge,
            vec![Some(EStat::Debt), Some(EStat::Ok), Some(EStat::Ok)]
        );
        assert_eq!(arch.node, vec![(1, true), (1, true), (0, false)]);
        let viol = evaluate(&g, &diags, &[false], &Params::from_query("p=viol"));
        assert_eq!(viol.edge, vec![Some(EStat::Debt), None, None]);
        assert_eq!(viol.node, vec![(1, true), (1, true), (0, false)]);
        let all = evaluate(&g, &diags, &[true], &Params::from_query("p=all"));
        assert_eq!(all.edge[0], Some(EStat::Accepted));
    }

    #[test]
    fn cycle_edges_are_violating_and_counted_and_focused() {
        let g = GraphView {
            // a -> b -> c -> a is the path; the b -> c edge is not on it.
            edges: vec![
                EdgeView {
                    diags: vec!["cy".into()],
                    ..graph().edges[0].clone()
                },
                EdgeView {
                    diags: vec![],
                    ..graph().edges[1].clone()
                },
                EdgeView {
                    diags: vec!["cy".into()],
                    ..graph().edges[2].clone()
                },
            ],
            ..graph()
        };
        let diags = vec![diag("cy", "deps/cycle", "error", &["a", "b", "c"])];
        for q in ["p=viol", "p=all", ""] {
            let ev = evaluate(&g, &diags, &[false], &Params::from_query(q));
            assert_eq!(ev.edge[0], Some(EStat::Debt), "{q}");
            assert_eq!(ev.edge[2], Some(EStat::Debt), "{q}");
            assert_ne!(ev.edge[1], Some(EStat::Debt), "{q}");
        }
        let viol = evaluate(&g, &diags, &[false], &Params::from_query("p=viol"));
        assert_eq!(viol.edge[1], None);
        let s = brickie_web_api::brick_scores(&g, &diags, &[false]);
        assert_eq!((s[0].viol_in, s[0].viol_out), (1, 1));
        assert_eq!((s[1].viol_in, s[1].viol_out), (1, 0));
        let f = evaluate(&g, &diags, &[false], &Params::from_query("item=cy"))
            .focus
            .unwrap();
        assert_eq!(f.hot, [0, 2].into());
    }

    #[test]
    fn selecting_a_brick_keeps_its_edges_and_a_finding_wins() {
        let g = graph();
        let diags = vec![diag("d1", "deps/deny", "error", &["a", "b"])];
        let p = Params::from_query("brick=c");
        let f = evaluate(&g, &diags, &[false], &p).focus.unwrap();
        assert_eq!(f.keep, [1, 2].into());
        let p = Params::from_query("brick=c&item=d1");
        let f = evaluate(&g, &diags, &[false], &p).focus.unwrap();
        assert!(f.finding && f.keep == [0].into() && f.bricks == [0, 1].into());
    }

    #[test]
    fn findings_list_is_scoped_by_selection() {
        let g = graph();
        let diags = vec![
            diag("d1", "deps/deny", "error", &["a", "b"]),
            diag("c1", "complexity/cognitive", "error", &["c"]),
        ];
        let p = Params::from_query("p=all");
        let ev = evaluate(&g, &diags, &[false, false], &p);
        assert_eq!(
            list_findings(&g, &diags, &[false, false], &p, &ev),
            vec![0, 1]
        );
        let p = Params::from_query("p=all&brick=c");
        let ev = evaluate(&g, &diags, &[false, false], &p);
        assert_eq!(list_findings(&g, &diags, &[false, false], &p, &ev), vec![1]);
    }
}
