//! `complexity/<metric>`: per-function limits.

use super::brick_of;
use super::codes::METRICS;
use crate::cache::Cache;
use crate::config::{Brick, ComplexityRule, Config, Level};
use crate::diag::{Finding, RuleRef, Span};
use std::path::Path;

struct Cx {
    brick: String,
    file: String,
    qname: String,
    metric: &'static str,
    value: u64,
    limit: u64,
    line: u32,
    col: Option<u32>,
    name_len: u32,
    rule_index: usize,
    rule: ComplexityRule,
}

type Metric = (&'static str, u64, fn(&ComplexityRule) -> Option<u64>);

fn metrics(func: &crate::cache::Function) -> [Metric; 6] {
    let [cognitive, cyclomatic, nesting, paths, params, lines] = METRICS;
    [
        (cognitive, func.cognitive.into(), |r| {
            r.cognitive.map(Into::into)
        }),
        (cyclomatic, func.cyclomatic.into(), |r| {
            r.cyclomatic.map(Into::into)
        }),
        (nesting, func.nesting.into(), |r| r.nesting.map(Into::into)),
        (paths, func.paths, |r| r.paths),
        (params, func.params.into(), |r| r.params.map(Into::into)),
        (lines, func.lines.into(), |r| r.lines.map(Into::into)),
    ]
}

/// The rule that sets a metric's limit for a brick and scope: the most specific rule that sets
/// it, the lower limit on a tie. Returns (limit, rule index, rule).
fn best_rule<'a>(
    cfg: &'a Config,
    brick: &Brick,
    scope: &str,
    limit_of: fn(&ComplexityRule) -> Option<u64>,
) -> Option<(u64, usize, &'a ComplexityRule)> {
    let mut best: Option<(u32, u64, usize, &ComplexityRule)> = None;
    for (i, r) in cfg.complexity.iter().enumerate() {
        let (Some(limit), Some(spec)) = (limit_of(r), r.bricks.specificity(brick)) else {
            continue;
        };
        if !r.scope.matches(scope) {
            continue;
        }
        if best.is_none_or(|(s, l, _, _)| spec > s || (spec == s && limit < l)) {
            best = Some((spec, limit, i, r));
        }
    }
    best.map(|(_, l, i, r)| (l, i, r))
}

/// Every metric of a function that has a limit under the rules that apply to its brick:
/// (metric, value, limit), whether or not the value exceeds it.
pub fn limits(
    cfg: &Config,
    brick: &Brick,
    func: &crate::cache::Function,
) -> Vec<(&'static str, u64, u64)> {
    metrics(func)
        .into_iter()
        .filter_map(|(m, v, f)| best_rule(cfg, brick, &func.scope, f).map(|(l, _, _)| (m, v, l)))
        .collect()
}

pub(super) fn complexity(
    cache: &Cache,
    cfg: &Config,
    root: &Path,
    out: &mut Vec<Box<dyn Finding>>,
) {
    if cfg.complexity.is_empty() {
        return;
    }
    for (path, f) in &cache.files {
        let mut source: Option<Vec<String>> = None;
        for func in &f.functions {
            let Some(brick) = func.module.as_deref().and_then(|m| brick_of(cache, cfg, m)) else {
                continue;
            };
            let metrics = metrics(func);
            for (metric, value, limit_of) in metrics {
                let best = best_rule(cfg, brick, &func.scope, limit_of);
                let Some((limit, rule_index, rule)) = best else {
                    continue;
                };
                if value <= limit {
                    continue;
                }
                let lines = source.get_or_insert_with(|| {
                    std::fs::read_to_string(root.join(path))
                        .map(|t| t.lines().map(str::to_string).collect())
                        .unwrap_or_default()
                });
                let col = lines
                    .get(func.line as usize - 1)
                    .and_then(|l| l.find(&format!("fn {}", func.name)))
                    .map(|b| b as u32 + 4);
                out.push(Box::new(Cx {
                    brick: brick.name.clone(),
                    file: path.clone(),
                    qname: func.qname.clone(),
                    metric,
                    value,
                    limit,
                    line: func.line,
                    col,
                    name_len: func.name.len() as u32,
                    rule_index,
                    rule: rule.clone(),
                }));
            }
        }
    }
}

impl Finding for Cx {
    fn bricks(&self) -> Vec<String> {
        vec![self.brick.clone()]
    }
    fn code(&self) -> String {
        format!("complexity/{}", self.metric)
    }
    fn level(&self) -> Level {
        self.rule.level
    }
    fn subject(&self) -> String {
        format!("{}::{}::{}", self.file, self.qname, self.metric)
    }
    fn message(&self) -> String {
        format!(
            "`{}` has {} {} (limit {})",
            self.qname, self.metric, self.value, self.limit
        )
    }
    fn spans(&self) -> Vec<Span> {
        vec![Span {
            file: self.file.clone(),
            line: self.line,
            col: self.col,
            len: Some(self.name_len),
            label: Some(format!(
                "{} over the limit by {}",
                self.metric,
                self.value - self.limit
            )),
        }]
    }
    fn rule(&self) -> Option<RuleRef> {
        Some(RuleRef {
            name: format!(
                "complexity[{}] {} <= {}",
                self.rule_index, self.metric, self.limit
            ),
            loc: Some(self.rule.loc.clone()),
        })
    }
    fn help(&self) -> Option<String> {
        let over = self.value - self.limit;
        Some(self.rule.help.clone().unwrap_or_else(|| match self.metric {
            "params" => format!("group {over} or more parameters into a struct"),
            "lines" => format!("split the function; it is {over} lines over the limit"),
            m => format!("return early or extract a function to cut {m} by at least {over}"),
        }))
    }
}
