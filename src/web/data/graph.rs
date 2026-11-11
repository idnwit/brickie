//! The brick graph rows.

use crate::check::Loaded;
use crate::diag;
use crate::rules;
use brickie_ui::*;
use std::collections::{BTreeMap, BTreeSet};

/// Bricks in config order with one row per tag (first tag of a brick, config order), and brick
/// edges aggregated by (from, to, kind). Test-scope edges are included and flagged. `diags` are
/// the ids of the `deps/*` findings of the edge's module edges. The layout runs in `brickie_ui`.
pub fn graph(l: &Loaded) -> GraphView {
    let mut rows: Vec<String> = Vec::new();
    let mut bricks = Vec::new();
    let mut heat = heat(l);
    for b in &l.cfg.bricks {
        let tag = b
            .tags
            .first()
            .cloned()
            .unwrap_or_else(|| "(untagged)".into());
        let row = match rows.iter().position(|r| *r == tag) {
            Some(i) => i,
            None => {
                rows.push(tag.clone());
                rows.len() - 1
            }
        };
        let (worst_ratio, hot) = heat.remove(&b.name).unwrap_or_default();
        bricks.push(BrickView {
            name: b.name.clone(),
            tag,
            row,
            tags: b.tags.clone(),
            members: b.members.clone(),
            worst_ratio,
            hot,
        });
    }
    let index: BTreeMap<&str, usize> = bricks
        .iter()
        .enumerate()
        .map(|(i, b)| (b.name.as_str(), i))
        .collect();
    let violating_ids: BTreeSet<&str> = l
        .diagnostics
        .iter()
        .filter(|d| d.code == rules::DEPS_DENY || d.code == rules::DEPS_DEFAULT)
        .map(|d| d.id.as_str())
        .collect();
    // (from, to, kind) -> (has a prod edge, finding ids)
    let mut agg: BTreeMap<(usize, usize, String), (bool, BTreeSet<String>)> = BTreeMap::new();
    let mb = &l.cache.bricks.module_brick;
    for e in l.cache.edges() {
        let (Some(fb), Some(tb)) = (mb.get(&e.from), mb.get(&e.to)) else {
            continue;
        };
        let (Some(&fi), Some(&ti)) = (index.get(fb.as_str()), index.get(tb.as_str())) else {
            continue;
        };
        if fi == ti {
            continue;
        }
        let entry = agg.entry((fi, ti, e.kind.clone())).or_default();
        entry.0 |= e.scope == "prod";
        for code in [rules::DEPS_DENY, rules::DEPS_DEFAULT] {
            let id = diag::finding_id(code, &rules::dep_subject(&e.from, &e.to, &e.kind, &e.scope));
            if violating_ids.contains(id.as_str()) {
                entry.1.insert(id);
            }
        }
    }
    // Brick edges on a cycle path carry the cycle finding (prod edges only: cycles are
    // computed on prod references).
    let cycle_hops = cycle_hops(&l.diagnostics, &index);
    let edges = agg
        .into_iter()
        .map(|((from, to, kind), (prod, mut ids))| {
            if prod {
                for id in cycle_hops.get(&(from, to)).into_iter().flatten() {
                    ids.insert(id.clone());
                }
            }
            EdgeView {
                from,
                to,
                kind,
                test: !prod,
                diags: ids.into_iter().collect(),
            }
        })
        .collect();
    GraphView {
        rows,
        bricks,
        edges,
    }
}

/// Brick index pairs on the path of each `deps/cycle` finding, with the finding ids.
fn cycle_hops(
    diags: &[crate::diag::Diagnostic],
    index: &BTreeMap<&str, usize>,
) -> BTreeMap<(usize, usize), Vec<String>> {
    let mut out: BTreeMap<(usize, usize), Vec<String>> = BTreeMap::new();
    for d in diags.iter().filter(|d| d.code == rules::DEPS_CYCLE) {
        for h in &d.edges {
            if let (Some(&f), Some(&t)) = (index.get(h.from.as_str()), index.get(h.to.as_str())) {
                out.entry((f, t)).or_default().push(d.id.clone());
            }
        }
    }
    out
}

/// Per brick: the worst value/limit ratio over its functions and the functions over a limit
/// (worst metric each), worst first. Limits come from the same rules `check` applies.
fn heat(l: &Loaded) -> BTreeMap<String, (f64, Vec<HotFn>)> {
    let mut out: BTreeMap<String, (f64, Vec<HotFn>)> = BTreeMap::new();
    for (path, f) in &l.cache.files {
        for func in &f.functions {
            let Some(brick) = func
                .module
                .as_deref()
                .and_then(|m| rules::brick_of(&l.cache, &l.cfg, m))
            else {
                continue;
            };
            let worst = rules::complexity_limits(&l.cfg, brick, func)
                .into_iter()
                .map(|(m, v, lim)| (m, v, lim, ratio(v, lim)))
                .max_by(|a, b| a.3.total_cmp(&b.3));
            let Some((metric, value, limit, r)) = worst else {
                continue;
            };
            let e = out.entry(brick.name.clone()).or_default();
            e.0 = e.0.max(r);
            if r > 1.0 {
                e.1.push(HotFn {
                    qname: func.qname.clone(),
                    file: path.clone(),
                    line: func.line,
                    metric: metric.into(),
                    value,
                    limit,
                    ratio: r,
                });
            }
        }
    }
    for (_, hot) in out.values_mut() {
        hot.sort_by(|a, b| b.ratio.total_cmp(&a.ratio).then(a.qname.cmp(&b.qname)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Brick, ComplexityRule, Config};

    #[test]
    fn cycle_hops_map_path_edges_to_the_finding() {
        use crate::diag::{DiagEdge, Diagnostic};
        let hop = |f: &str, t: &str| DiagEdge {
            from: f.into(),
            to: t.into(),
            span: None,
        };
        let d = Diagnostic {
            id: "cyc1".into(),
            level: crate::config::Level::Error,
            code: rules::DEPS_CYCLE.into(),
            message: String::new(),
            span: None,
            spans: vec![],
            rule: None,
            help: None,
            notes: vec![],
            bricks: vec!["a".into(), "b".into()],
            edges: vec![hop("a", "b"), hop("b", "a"), hop("b", "zzz")],
            accepted: false,
            accepted_reason: None,
        };
        let index: BTreeMap<&str, usize> = [("a", 0), ("b", 1), ("c", 2)].into();
        let m = cycle_hops(&[d], &index);
        assert_eq!(m.len(), 2);
        assert_eq!(m[&(0, 1)], vec!["cyc1".to_string()]);
        assert_eq!(m[&(1, 0)], vec!["cyc1".to_string()]);
    }

    #[test]
    fn complexity_limits_use_the_most_specific_rule() {
        let rule = |bricks: &str, cognitive| -> ComplexityRule {
            let t = format!("bricks = \"{bricks}\"\ncognitive = {cognitive}");
            toml::from_str(&t).unwrap()
        };
        let brick = Brick {
            name: "core".into(),
            tags: vec!["engine".into()],
            members: vec![],
        };
        let cfg = Config {
            bricks: vec![brick.clone()],
            complexity: vec![rule("*", 15), rule("core", 30)],
            ..Default::default()
        };
        let func: crate::cache::Function = serde_json::from_value(serde_json::json!({
            "qname": "f", "name": "f", "module": null, "line": 1, "end_line": 5, "scope": "prod", "cognitive": 27,
            "cyclomatic": 1, "nesting": 1, "paths": 1, "params": 1, "lines": 5
        }))
        .unwrap();
        let limits = rules::complexity_limits(&cfg, &brick, &func);
        assert_eq!(limits, vec![("cognitive", 27, 30)]);
    }
}
