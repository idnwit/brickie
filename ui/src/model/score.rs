//! Brick scores: the one place that turns findings and function metrics into numbers. The
//! server embeds the static part (per-function complexity ratios in `BrickView`); the open
//! finding counts depend on review state, so the page computes them with [`brick_scores`].

use super::{BrickView, DiagView, GraphView};
use serde::{Deserialize, Serialize};

/// Weight of an open warning relative to an open error.
pub const WARN_WEIGHT: f64 = 0.25;

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
pub const CATEGORIES: [&str; 4] = ["deps", "cycle", "complexity", "stale"];

/// Index into [`CATEGORIES`] of a diagnostic code.
pub fn category(code: &str) -> usize {
    match code {
        "deps/cycle" => 1,
        c if c.starts_with("deps/") => 0,
        c if c.starts_with("complexity/") => 2,
        _ => 3,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BrickScore {
    pub errors: u32,
    pub warns: u32,
    /// Open errors and warnings per [`CATEGORIES`].
    pub by_cat: [u32; 4],
    /// Sum over functions of `max(0, worst ratio - 1)`.
    pub excess: f64,
    /// `errors + 0.25 * warns + excess`.
    pub score: f64,
    pub edges_in: u32,
    pub edges_out: u32,
    /// Edges with an open deps finding.
    pub viol_in: u32,
    pub viol_out: u32,
}

impl BrickScore {
    pub fn clean(&self) -> bool {
        self.errors == 0 && self.warns == 0
    }
}

/// Complexity excess of one brick: the sum of `ratio - 1` over its hot functions.
pub fn excess(b: &BrickView) -> f64 {
    b.hot.iter().map(|f| (f.ratio - 1.0).max(0.0)).sum()
}

/// Scores of all bricks; `accepted[i]` is the review state of `diags[i]`. Only open (not
/// accepted) findings of level `error` or `warn` count, whatever layers are shown.
pub fn brick_scores(g: &GraphView, diags: &[DiagView], accepted: &[bool]) -> Vec<BrickScore> {
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
    let open = |i: usize| !accepted.get(i).copied().unwrap_or(false);
    for (i, d) in diags.iter().enumerate() {
        let err = d.level == "error";
        if !open(i) || !(err || d.level == "warn") {
            continue;
        }
        for b in d
            .bricks
            .iter()
            .filter_map(|n| g.bricks.iter().position(|b| b.name == *n))
        {
            let s = &mut out[b];
            if err {
                s.errors += 1;
                s.score += 1.0;
            } else {
                s.warns += 1;
                s.score += WARN_WEIGHT;
            }
            s.by_cat[category(&d.code)] += 1;
        }
    }
    for e in &g.edges {
        let viol = e.diags.iter().any(|id| {
            diags
                .iter()
                .position(|d| d.id == *id)
                .is_some_and(|i| open(i) && matches!(diags[i].level.as_str(), "error" | "warn"))
        });
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
            }],
        }
    }

    #[test]
    fn ratio_is_value_over_limit() {
        assert!((ratio(27, 15) - 1.8).abs() < 1e-9);
        assert_eq!(ratio(3, 0), 3.0);
    }

    #[test]
    fn score_adds_errors_warnings_and_excess() {
        let g = graph();
        let diags = vec![
            diag("d1", "deps/deny", "error", &["a", "b"]),
            diag("w1", "complexity/cognitive", "warn", &["a"]),
            diag("c1", "deps/cycle", "error", &["a"]),
            diag("i1", "stale/file", "info", &["a"]),
        ];
        let s = brick_scores(&g, &diags, &[false; 4]);
        assert_eq!((s[0].errors, s[0].warns), (2, 1));
        assert_eq!(s[0].by_cat, [1, 1, 1, 0]);
        // 2 errors + 0.25 warn + (0.8 + 0.2) excess
        assert!((s[0].score - 3.25).abs() < 1e-9);
        assert_eq!((s[1].errors, s[1].score), (1, 1.0));
        assert_eq!((s[0].edges_out, s[0].viol_out, s[1].viol_in), (1, 1, 1));
    }

    #[test]
    fn accepted_findings_do_not_count() {
        let g = graph();
        let diags = vec![diag("d1", "deps/deny", "error", &["a", "b"])];
        let s = brick_scores(&g, &diags, &[true]);
        assert!(s[1].clean() && s[1].score == 0.0 && s[1].viol_in == 0);
        assert!((s[0].score - 1.0).abs() < 1e-9);
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
