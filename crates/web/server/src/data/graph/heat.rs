//! Per node of the tree: the worst value/limit ratio of the functions below it and the
//! functions over a limit (worst metric each), worst first. Limits are the rules `check` applies.

use brickie_check::check as rules;
use brickie_check::check::Loaded;
use brickie_check::config::level_name;
use brickie_ui::{HotFn, ratio};
use std::collections::BTreeMap;

/// Keyed by node level path (`root/web/api`); a composite collects its whole subtree.
pub fn heat(l: &Loaded) -> BTreeMap<String, (f64, Vec<HotFn>)> {
    let mut out: BTreeMap<String, (f64, Vec<HotFn>)> = BTreeMap::new();
    for (path, f) in &l.cache.files {
        for func in &f.functions {
            let Some(leaf) = func.module.as_deref().and_then(|m| l.resolved.path(m)) else {
                continue;
            };
            let worst = rules::complexity_limits(&l.cfg, leaf, func)
                .into_iter()
                .map(|(m, v, lim)| (m, v, lim, ratio(v, lim)))
                .max_by(|a, b| a.3.total_cmp(&b.3));
            let Some((metric, value, limit, r)) = worst else {
                continue;
            };
            for n in 1..=leaf.len() {
                let e = out.entry(level_name(&leaf[..n])).or_default();
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
    }
    for (_, hot) in out.values_mut() {
        hot.sort_by(|a, b| b.ratio.total_cmp(&a.ratio).then(a.qname.cmp(&b.qname)));
    }
    out
}
