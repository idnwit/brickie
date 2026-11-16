//! Brick scores: the one place that turns findings and function metrics into numbers. The
//! server embeds the static part (per-function complexity ratios in `BrickView`); the
//! finding counts depend on the layers shown, so the page computes them with [`brick_scores`].

use super::{BrickView, DiagView, GraphView};
use serde::{Deserialize, Serialize};

/// A function over its worst limit: the metric with the highest value/limit ratio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotFn {
    pub qname: String,
    pub file: String,
    pub line: u32,
    pub metric: String,
    pub value: u64,
    pub limit: u64,
    /// `value / limit`, greater than 1 for every entry of `BrickView::hot`.
    pub ratio: f64,
}

/// `value / limit` (a zero limit counts as 1).
pub fn ratio(value: u64, limit: u64) -> f64 {
    value as f64 / limit.max(1) as f64
}

/// The finding categories shown in the scorecard, in display order.
pub const CATEGORIES: [&str; 5] = ["deps", "cycle", "complexity", "stale", "interface"];

/// Index into [`CATEGORIES`] of a diagnostic code.
fn category(code: &str) -> usize {
    match code {
        "deps/cycle" => 1,
        c if c.starts_with("deps/") => 0,
        c if c.starts_with("complexity/") => 2,
        c if c.starts_with("interface/") => 4,
        _ => 3,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BrickScore {
    pub errors: u32,
    /// Findings per [`CATEGORIES`].
    pub by_cat: [u32; 5],
    /// Sum over functions of `max(0, worst ratio - 1)`.
    pub excess: f64,
    /// `errors + excess`.
    pub score: f64,
    pub edges_in: u32,
    pub edges_out: u32,
    /// Edges with a finding.
    pub viol_in: u32,
    pub viol_out: u32,
}

impl BrickScore {
    pub fn clean(&self) -> bool {
        self.errors == 0
    }
}

/// Complexity excess of one brick: the sum of `ratio - 1` over its hot functions.
fn excess(b: &BrickView) -> f64 {
    b.hot.iter().map(|f| (f.ratio - 1.0).max(0.0)).sum()
}

/// Scores of all bricks: every finding counts, whatever layers are shown.
pub fn brick_scores(g: &GraphView, diags: &[DiagView]) -> Vec<BrickScore> {
    let mut out: Vec<BrickScore> = g
        .bricks
        .iter()
        .map(|b| {
            let x = excess(b);
            BrickScore {
                excess: x,
                score: x,
                ..Default::default()
            }
        })
        .collect();
    for d in diags {
        for b in super::diag_bricks(g, d) {
            let s = &mut out[b];
            s.errors += 1;
            s.score += 1.0;
            s.by_cat[category(&d.code)] += 1;
        }
    }
    // Frame edges repeat boundary edges (another drawing of the same edges): not counted.
    for e in g.edges.iter().filter(|e| !e.frame()) {
        let viol = e.diags.iter().any(|id| diags.iter().any(|d| d.id == *id));
        out[e.from].edges_out += 1;
        out[e.to].edges_in += 1;
        if viol {
            out[e.from].viol_out += 1;
            out[e.to].viol_in += 1;
        }
    }
    out
}

/// Score as shown: at most two decimals, no trailing zeros.
pub fn fmt_score(s: f64) -> String {
    let t = format!("{s:.2}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Heat bar fill in 0..=1 for a worst ratio: ratio 2 or more is full.
pub fn heat_len(worst: f64) -> f64 {
    (worst / 2.0).clamp(0.0, 1.0)
}

/// Heat level: 0 within the limit (below 0.8), 1 close to it, 2 over it, 3 more than 1.5x over.
pub fn heat_level(worst: f64) -> u8 {
    match worst {
        w if w > 1.5 => 3,
        w if w > 1.0 => 2,
        w if w >= 0.8 => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EdgeView;

    fn hot(ratio: f64) -> HotFn {
        HotFn {
            qname: "f".into(),
            file: "a.rs".into(),
            line: 1,
            metric: "cognitive".into(),
            value: 0,
            limit: 0,
            ratio,
        }
    }

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
        let b = |n: &str, hot: Vec<HotFn>| BrickView {
            name: n.into(),
            tag: "t".into(),
            row: 0,
            tags: vec!["t".into()],
            members: vec![],
            worst_ratio: hot.iter().map(|f| f.ratio).fold(0.0, f64::max),
            hot,
            ..Default::default()
        };
        GraphView {
            rows: vec!["t".into()],
            bricks: vec![b("a", vec![hot(1.8), hot(1.2)]), b("b", vec![])],
            edges: vec![EdgeView {
                from: 0,
                to: 1,
                kind: "uses".into(),
                test: false,
                diags: vec!["d1".into()],
                ..Default::default()
            }],
        }
    }

    #[test]
    fn ratio_is_value_over_limit() {
        assert!((ratio(27, 15) - 1.8).abs() < 1e-9);
        assert_eq!(ratio(3, 0), 3.0);
    }

    #[test]
    fn score_adds_errors_and_excess() {
        let g = graph();
        let diags = vec![
            diag("d1", "deps/deny", &["a", "b"]),
            diag("w1", "complexity/cognitive", &["a"]),
            diag("c1", "deps/cycle", &["a"]),
            diag("i1", "stale/file", &["a"]),
        ];
        let s = brick_scores(&g, &diags);
        assert_eq!(s[0].errors, 4);
        assert_eq!(s[0].by_cat, [1, 1, 1, 1, 0]);
        // 4 findings + (0.8 + 0.2) excess
        assert!((s[0].score - 5.0).abs() < 1e-9);
        assert_eq!((s[1].errors, s[1].score), (1, 1.0));
        assert_eq!((s[0].edges_out, s[0].viol_out, s[1].viol_in), (1, 1, 1));
    }

    #[test]
    fn formatting_and_heat() {
        assert_eq!(fmt_score(3.25), "3.25");
        assert_eq!(fmt_score(2.0), "2");
        assert_eq!(heat_len(3.0), 1.0);
        assert_eq!(heat_len(0.5), 0.25);
        assert_eq!([0.5, 0.9, 1.2, 1.8].map(heat_level), [0, 1, 2, 3]);
    }
}
