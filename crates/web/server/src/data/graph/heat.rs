//! Per node of the tree: the worst value/limit ratio of the functions below it and the
//! functions over a limit (worst metric each), worst first. Limits are the rules `check` applies.

use brickie_check::check as rules;
use brickie_check::check::Loaded;
use brickie_check::config::level_name;
use brickie_web_api::{HotFn, ratio};
use std::collections::BTreeMap;

/// The worst ratio of a node and its functions over a limit.
type Node = (f64, Vec<HotFn>);

/// Keyed by node level path (`root/web/api`); a composite collects its whole subtree.
pub fn heat(l: &Loaded) -> BTreeMap<String, Node> {
    let mut out: BTreeMap<String, Node> = BTreeMap::new();
    for (path, f) in &l.cache.files {
        for func in &f.functions {
            let Some(leaf) = func.module.as_deref().and_then(|m| l.resolved.path(m)) else {
                continue;
            };
            let Some(hot) = worst_limit(l, leaf, func, path) else {
                continue;
            };
            add_to_nodes(&mut out, leaf, &hot);
        }
    }
    for (_, hot) in out.values_mut() {
        hot.sort_by(|a, b| b.ratio.total_cmp(&a.ratio).then(a.qname.cmp(&b.qname)));
    }
    out
}

/// Count `hot` on every node from the root to `leaf`; only a ratio over 1 is listed.
fn add_to_nodes(out: &mut BTreeMap<String, Node>, leaf: &[String], hot: &HotFn) {
    for n in 1..=leaf.len() {
        let e = out.entry(level_name(&leaf[..n])).or_default();
        e.0 = e.0.max(hot.ratio);
        if hot.ratio > 1.0 {
            e.1.push(hot.clone());
        }
    }
}

/// The metric of `func` (in leaf `leaf`, file `path`) closest to or furthest over its limit.
fn worst_limit(
    l: &Loaded,
    leaf: &[String],
    func: &brickie_facts::Function,
    path: &str,
) -> Option<HotFn> {
    rules::complexity_limits(&l.cfg, leaf, func)
        .into_iter()
        .map(|(m, v, lim)| HotFn {
            qname: func.qname.clone(),
            file: path.to_string(),
            line: func.line,
            metric: m.into(),
            value: v,
            limit: lim,
            ratio: ratio(v, lim),
        })
        .max_by(|a, b| a.ratio.total_cmp(&b.ratio))
}
