//! What the page shows for the current params: which findings pass the layers, which edges are
//! drawn and how, the per-brick finding counts and the selection focus. Pure; the app wraps it in
//! a memo. Ports `fOn`/`vOn`, `violIds`, `edgeOn`, `fSpec` and `focusSpec`.

use super::*;
use brickie_web_api::*;
use std::collections::BTreeSet;

/// Effective status of a drawn edge (`edgeOn`): `ok` or `debt` (violating).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EStat {
    Ok,
    Debt,
}

/// The result of [`evaluate`]: what the page shows for the current params.
#[derive(Debug, Clone)]
pub struct Eval {
    pub sel: Sel,
    /// `vOn` per diagnostic.
    pub von: Vec<bool>,
    /// Drawn status per edge, `None` when no enabled layer shows it.
    pub edge: Vec<Option<EStat>>,
    /// Per brick: findings counted by `vOn`.
    pub node: Vec<usize>,
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
}

impl EdgeFacts {
    /// The drawn status, `None` when the edge is hidden.
    fn stat(&self) -> Option<EStat> {
        if self.viol_preset {
            return self.in_viol.then_some(EStat::Debt);
        }
        let debt = self.dbt && self.dbt_on;
        if (self.test && !self.tests_on && !debt) || !(self.edges_on || debt) {
            return None;
        }
        Some(if debt { EStat::Debt } else { EStat::Ok })
    }
}

/// The inputs of one evaluation and the per-diagnostic `vOn`, shared by its parts.
struct EvalIn<'a> {
    g: &'a GraphView,
    diags: &'a [DiagView],
    p: &'a Params,
    lay: Lay,
    von: Vec<bool>,
}

impl<'a> EvalIn<'a> {
    fn new(g: &'a GraphView, diags: &'a [DiagView], p: &'a Params) -> Self {
        let lay = Lay::from_params(p);
        let von = diags.iter().map(|d| v_on(&lay, p, &d.code)).collect();
        EvalIn {
            g,
            diags,
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
        }
        .stat()
    }

    /// Per brick: shown findings.
    fn node_counts(&self) -> Vec<usize> {
        let mut node = vec![0usize; self.g.bricks.len()];
        for (d, _) in self.diags.iter().zip(&self.von).filter(|(_, v)| **v) {
            for b in diag_bricks(self.g, d) {
                node[b] += 1;
            }
        }
        node
    }
}

/// Evaluates the graph `g` and its findings `diags` under `p`: the selection, which findings
/// pass the layers, how each edge is drawn, per-brick finding counts and the focus. Pure and
/// cheap enough to re-run in a memo on every params change.
pub fn evaluate(g: &GraphView, diags: &[DiagView], p: &Params) -> Eval {
    let ev = EvalIn::new(g, diags, p);
    let sel = Sel::from_params(p, g);
    let viol = ev.viol_edges();
    let edge = (0..g.edges.len()).map(|i| ev.edge_stat(i, &viol)).collect();
    let node = ev.node_counts();
    let focus = focus_of(g, &sel, p, diags);
    Eval {
        sel,
        von: ev.von,
        edge,
        node,
        focus,
    }
}

/// `listFindings` (without the text filter): the diagnostics shown in the left list.
pub fn list_findings(g: &GraphView, diags: &[DiagView], p: &Params, ev: &Eval) -> Vec<usize> {
    (0..diags.len())
        .filter(|&i| {
            let d = &diags[i];
            ev.von[i]
                && in_selection(g, d, &ev.sel)
                && (p.below.is_empty()
                    || under(d.brick_level.as_deref().unwrap_or("root"), p.level_path()))
                && (p.grp.is_empty() || group_sub(&d.code).0 == p.grp)
                && code_matches(&d.code, &p.code)
        })
        .collect()
}

/// Whether finding `d` belongs to the selection: a brick scopes to the findings touching it, a
/// link to its own findings only, nothing to every finding.
fn in_selection(g: &GraphView, d: &DiagView, sel: &Sel) -> bool {
    match sel {
        Sel::None => true,
        Sel::Brick(b) => diag_bricks(g, d).contains(b),
        Sel::Edge(e) => g.edges[*e].diags.contains(&d.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(id: &str, code: &str, bricks: &[&str]) -> DiagView {
        DiagView {
            id: id.into(),
            code: code.into(),
            message: String::new(),
            span: None,
            spans: vec![],
            rule: None,
            help: None,
            notes: vec![],
            bricks: bricks.iter().map(|s| s.to_string()).collect(),
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

    /// One edge a -> b (kind `uses`) with `found` findings on it.
    fn edge_stat_of(q: &str, test: bool, found: usize) -> Option<EStat> {
        let mut g = graph();
        g.edges = vec![EdgeView {
            test,
            diags: (0..found).map(|i| format!("d{i}")).collect(),
            ..graph().edges[1].clone()
        }];
        let diags: Vec<DiagView> = (0..found)
            .map(|i| diag(&format!("d{i}"), "deps/unlinked", &["a", "b"]))
            .collect();
        evaluate(&g, &diags, &Params::from_query(q)).edge[0]
    }

    #[test]
    fn edge_status_table() {
        use EStat::{Debt as D, Ok as O};
        // (query, test-only edge, number of findings on the edge, drawn status)
        let table = [
            ("", false, 0, Some(O)),
            ("", true, 0, None),
            ("lay=%2Btests", true, 0, Some(O)),
            ("", true, 1, Some(D)),
            ("", false, 2, Some(D)),
            ("lay=-edges", false, 0, None),
            ("lay=-edges", false, 1, Some(D)),
            ("lay=-deps", false, 1, Some(O)),
            ("lay=-deps", true, 1, None),
            ("lay=-deps,-edges", false, 1, None),
            ("lay=-k-uses", false, 1, None),
            ("p=all", true, 0, None),
            ("p=viol", false, 0, None),
            ("p=viol", false, 1, Some(D)),
            ("p=viol", true, 1, Some(D)),
            ("p=viol&vs=cx", false, 1, None),
            ("p=viol&vs=deps", false, 1, Some(D)),
            ("p=viol&lay=-deps", false, 1, None),
            ("p=viol&lay=-k-uses", false, 1, None),
        ];
        for (q, test, found, want) in table {
            assert_eq!(
                edge_stat_of(q, test, found),
                want,
                "{q} test={test} found={found}"
            );
        }
    }

    #[test]
    fn a_selected_link_lists_only_its_own_findings() {
        let g = graph();
        // d2 touches both ends of link 0 but is not one of its findings.
        let diags = vec![
            diag("d1", "deps/unlinked", &["a", "b"]),
            diag("d2", "deps/unlinked", &["a", "b"]),
        ];
        let list = |sel: Sel| {
            let p = sel.apply(&Params::default(), &g);
            let ev = evaluate(&g, &diags, &p);
            list_findings(&g, &diags, &p, &ev)
        };
        assert_eq!(list(Sel::None), vec![0, 1]);
        assert_eq!(list(Sel::Brick(0)), vec![0, 1]);
        assert_eq!(list(Sel::Edge(0)), vec![0]);
        assert!(list(Sel::Edge(1)).is_empty());
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
        let drawn = |q: &str| evaluate(&g, &[], &Params::from_query(q)).edge;
        let ok = Some(EStat::Ok);
        assert_eq!(drawn(""), vec![ok, ok, None]);
        assert_eq!(drawn("bnd=frame"), vec![ok, None, ok]);
    }

    #[test]
    fn architecture_shows_all_edges_with_violating_ones_marked() {
        let g = graph();
        let diags = vec![diag("d1", "deps/unlinked", &["a", "b"])];
        let arch = evaluate(&g, &diags, &Params::default());
        // Architecture draws every edge and marks the violating one; complexity and stale
        // findings stay hidden.
        assert_eq!(
            arch.edge,
            vec![Some(EStat::Debt), Some(EStat::Ok), Some(EStat::Ok)]
        );
        assert_eq!(arch.node, vec![1, 1, 0]);
        let viol = evaluate(&g, &diags, &Params::from_query("p=viol"));
        assert_eq!(viol.edge, vec![Some(EStat::Debt), None, None]);
        assert_eq!(viol.node, vec![1, 1, 0]);
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
        let diags = vec![diag("cy", "deps/cycle", &["a", "b", "c"])];
        for q in ["p=viol", "p=all", ""] {
            let ev = evaluate(&g, &diags, &Params::from_query(q));
            assert_eq!(ev.edge[0], Some(EStat::Debt), "{q}");
            assert_eq!(ev.edge[2], Some(EStat::Debt), "{q}");
            assert_ne!(ev.edge[1], Some(EStat::Debt), "{q}");
        }
        let viol = evaluate(&g, &diags, &Params::from_query("p=viol"));
        assert_eq!(viol.edge[1], None);
        let s = brickie_web_api::brick_scores(&g, &diags);
        assert_eq!((s[0].viol_in, s[0].viol_out), (1, 1));
        assert_eq!((s[1].viol_in, s[1].viol_out), (1, 0));
        let f = evaluate(&g, &diags, &Params::from_query("item=cy"))
            .focus
            .unwrap();
        assert_eq!(f.hot, [0, 2].into());
    }

    #[test]
    fn selecting_a_brick_keeps_its_edges_and_a_finding_wins() {
        let g = graph();
        let diags = vec![diag("d1", "deps/unlinked", &["a", "b"])];
        let p = Params::from_query("brick=c");
        let f = evaluate(&g, &diags, &p).focus.unwrap();
        assert_eq!(f.keep, [1, 2].into());
        let p = Params::from_query("brick=c&item=d1");
        let f = evaluate(&g, &diags, &p).focus.unwrap();
        assert!(f.finding && f.keep == [0].into() && f.bricks == [0, 1].into());
    }

    #[test]
    fn findings_list_is_scoped_by_selection() {
        let g = graph();
        let diags = vec![
            diag("d1", "deps/unlinked", &["a", "b"]),
            diag("c1", "complexity/cognitive", &["c"]),
        ];
        let p = Params::from_query("p=all");
        let ev = evaluate(&g, &diags, &p);
        assert_eq!(list_findings(&g, &diags, &p, &ev), vec![0, 1]);
        let p = Params::from_query("p=all&brick=c");
        let ev = evaluate(&g, &diags, &p);
        assert_eq!(list_findings(&g, &diags, &p, &ev), vec![1]);
    }
}
