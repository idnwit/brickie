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

/// The facts that decide how one edge is drawn (`edgeOn` after the layer and boundary gates).
struct EdgeFacts {
    viol_preset: bool,
    /// A shown deps finding points at it (`violIds`).
    in_viol: bool,
    test: bool,
    tests_on: bool,
    edges_on: bool,
    /// It carries findings, and at least one finding's layer is on.
    dbt: bool,
    dbt_on: bool,
    all_accepted: bool,
}

impl EdgeFacts {
    /// The drawn status, `None` when the edge is hidden.
    fn stat(&self) -> Option<EStat> {
        let marked = if self.all_accepted {
            EStat::Accepted
        } else {
            EStat::Debt
        };
        if self.viol_preset {
            return self.in_viol.then_some(marked);
        }
        let debt = self.dbt && self.dbt_on;
        if (self.test && !self.tests_on && !debt) || !(self.edges_on || debt) {
            return None;
        }
        Some(if debt { marked } else { EStat::Ok })
    }
}

/// The inputs of one evaluation and the per-diagnostic `vOn`, shared by its parts.
struct EvalIn<'a> {
    g: &'a GraphView,
    diags: &'a [DiagView],
    accepted: &'a [bool],
    p: &'a Params,
    lay: Lay,
    von: Vec<bool>,
}

impl<'a> EvalIn<'a> {
    fn new(g: &'a GraphView, diags: &'a [DiagView], accepted: &'a [bool], p: &'a Params) -> Self {
        let lay = Lay::from_params(p);
        let von = diags
            .iter()
            .map(|d| v_on(&lay, p, &d.code, &d.level))
            .collect();
        EvalIn {
            g,
            diags,
            accepted,
            p,
            lay,
            von,
        }
    }

    fn by_id(&self, id: &str) -> Option<usize> {
        self.diags.iter().position(|d| d.id == id)
    }

    /// `violIds`: edges a deps finding points at (violating edges, cycle members).
    fn viol_edges(&self) -> BTreeSet<usize> {
        if self.p.preset() != "viol" || !matches!(self.p.vscope(), "all" | "deps") {
            return BTreeSet::new();
        }
        let shown = |e: &EdgeView| {
            e.diags
                .iter()
                .filter_map(|id| self.by_id(id))
                .any(|c| self.von[c])
        };
        (0..self.g.edges.len())
            .filter(|&i| shown(&self.g.edges[i]))
            .collect()
    }

    /// The drawn status of edge `i`, `None` when no enabled layer shows it.
    fn edge_stat(&self, i: usize, viol: &BTreeSet<usize>) -> Option<EStat> {
        let e = &self.g.edges[i];
        // One of the two boundary drawings: to the children or aggregated at the frame.
        let hidden = if self.p.to_frame() {
            is_boundary(self.g, e)
        } else {
            e.frame()
        };
        if hidden || !self.lay.on(&format!("k-{}", e.kind)) {
            return None;
        }
        let cards: Vec<usize> = e.diags.iter().filter_map(|id| self.by_id(id)).collect();
        EdgeFacts {
            viol_preset: self.p.preset() == "viol",
            in_viol: viol.contains(&i),
            test: e.test,
            tests_on: self.lay.on("tests"),
            edges_on: self.lay.on("edges"),
            dbt: !cards.is_empty(),
            dbt_on: cards.iter().any(|&c| f_on(&self.lay, &self.diags[c].code)),
            all_accepted: !cards.is_empty() && cards.iter().all(|&c| self.accepted[c]),
        }
        .stat()
    }

    /// Per brick: open shown findings and whether one is an error, and whether it has shown
    /// findings that are all accepted.
    fn node_counts(&self) -> (Vec<(usize, bool)>, Vec<bool>) {
        let mut node = vec![(0usize, false); self.g.bricks.len()];
        let mut node_acc = vec![false; self.g.bricks.len()];
        for (c, d) in self.diags.iter().enumerate().filter(|&(c, _)| self.von[c]) {
            for b in diag_bricks(self.g, d) {
                if self.accepted[c] {
                    node_acc[b] = true;
                } else {
                    node[b].0 += 1;
                    node[b].1 |= d.level == "error";
                }
            }
        }
        (node, node_acc)
    }
}

pub fn evaluate(g: &GraphView, diags: &[DiagView], accepted: &[bool], p: &Params) -> Eval {
    let ev = EvalIn::new(g, diags, accepted, p);
    let sel = Sel::from_params(p, g);
    let viol = ev.viol_edges();
    let edge = (0..g.edges.len()).map(|i| ev.edge_stat(i, &viol)).collect();
    let (node, node_acc) = ev.node_counts();
    let focus = focus_of(g, &sel, p, diags);
    Eval {
        sel,
        von: ev.von,
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

    /// One edge a -> b (kind `uses`) with the given findings (level, accepted) on it.
    fn edge_stat_of(q: &str, test: bool, found: &[(&str, bool)]) -> Option<EStat> {
        let mut g = graph();
        g.edges = vec![EdgeView {
            test,
            diags: (0..found.len()).map(|i| format!("d{i}")).collect(),
            ..graph().edges[1].clone()
        }];
        let diags: Vec<DiagView> = found
            .iter()
            .enumerate()
            .map(|(i, (lv, _))| diag(&format!("d{i}"), "deps/deny", lv, &["a", "b"]))
            .collect();
        let acc: Vec<bool> = found.iter().map(|f| f.1).collect();
        evaluate(&g, &diags, &acc, &Params::from_query(q)).edge[0]
    }

    #[test]
    fn edge_status_table() {
        use EStat::{Accepted as A, Debt as D, Ok as O};
        let err = ("error", false);
        let acc = ("error", true);
        let warn = ("warn", false);
        // (query, test-only edge, findings on the edge, drawn status)
        let table = [
            ("", false, vec![], Some(O)),
            ("", true, vec![], None),
            ("lay=%2Btests", true, vec![], Some(O)),
            ("", true, vec![err], Some(D)),
            ("", true, vec![acc], Some(A)),
            ("", false, vec![warn], Some(D)),
            ("", false, vec![acc, err], Some(D)),
            ("", false, vec![acc, acc], Some(A)),
            ("lay=-edges", false, vec![], None),
            ("lay=-edges", false, vec![err], Some(D)),
            ("lay=-deps", false, vec![err], Some(O)),
            ("lay=-deps", true, vec![err], None),
            ("lay=-deps,-edges", false, vec![err], None),
            ("lay=-k-uses", false, vec![err], None),
            ("p=all", true, vec![], None),
            ("p=all", false, vec![warn], Some(D)),
            ("p=viol", false, vec![], None),
            ("p=viol", false, vec![err], Some(D)),
            ("p=viol", true, vec![err], Some(D)),
            ("p=viol", false, vec![acc], Some(A)),
            ("p=viol", false, vec![acc, err], Some(D)),
            ("p=viol", false, vec![warn], None),
            ("p=viol&level=warn", false, vec![warn], Some(D)),
            ("p=viol&vs=cx", false, vec![err], None),
            ("p=viol&vs=deps", false, vec![err], Some(D)),
            ("p=viol&lay=-deps", false, vec![err], None),
            ("p=viol&lay=-k-uses", false, vec![err], None),
        ];
        for (q, test, found, want) in table {
            assert_eq!(
                edge_stat_of(q, test, &found),
                want,
                "{q} test={test} {found:?}"
            );
        }
    }

    #[test]
    fn edge_drawing_follows_the_boundary_mode() {
        let mut g = graph();
        g.bricks[2].ghost = true;
        let e = |from, to, under: Vec<usize>| EdgeView {
            from,
            to,
            kind: "uses".into(),
            under,
            ..Default::default()
        };
        // 0: inner edge, 1: boundary edge to the children, 2: aggregated edge to the frame.
        g.edges = vec![e(0, 1, vec![]), e(1, 2, vec![]), e(2, 1, vec![1])];
        let drawn = |q: &str| evaluate(&g, &[], &[], &Params::from_query(q)).edge;
        let ok = Some(EStat::Ok);
        assert_eq!(drawn(""), vec![ok, ok, None]);
        assert_eq!(drawn("bnd=frame"), vec![ok, None, ok]);
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
