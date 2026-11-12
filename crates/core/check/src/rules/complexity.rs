//! `complexity/<metric>`: per-function limits. The rules of every level on a function's leaf
//! path apply (each level's rules select its own child on the path); per metric the tightest
//! limit wins, so a nested config can only tighten.

use crate::config::{Brick, ComplexityRule, Config, Level, Resolved, level_name};
use crate::diag::{Finding, RuleRef, Span};
use brickie_facts::Cache;
use brickie_model::codes::METRICS;
use std::path::Path;

struct Cx {
    /// Leaf brick name and the level it is a child of (`root/web`).
    brick: String,
    level: String,
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

fn metrics(func: &brickie_facts::Function) -> [Metric; 6] {
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

/// The limit of a metric for a leaf path and scope: on each level of the path, the most
/// specific rule of that level that sets it for the child on the path; across levels the
/// lowest limit. Returns (limit, rule index within its file's level, rule).
fn best_rule<'a>(
    cfg: &'a Config,
    path: &[String],
    scope: &str,
    limit_of: fn(&ComplexityRule) -> Option<u64>,
) -> Option<(u64, usize, &'a ComplexityRule)> {
    (0..path.len())
        .filter_map(|i| {
            let level = cfg.node(&path[..i])?;
            level_rule(level, level.brick(&path[i])?, scope, limit_of)
        })
        .min_by_key(|(l, _, _)| *l)
}

/// The rule of one level that sets a metric's limit for a brick and scope: the most specific
/// rule that sets it, the lower limit on a tie.
fn level_rule<'a>(
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

/// Every metric of a function that has a limit under the rules that apply to its leaf path:
/// (metric, value, limit), whether or not the value exceeds it.
pub fn limits(
    cfg: &Config,
    leaf: &[String],
    func: &brickie_facts::Function,
) -> Vec<(&'static str, u64, u64)> {
    metrics(func)
        .into_iter()
        .filter_map(|(m, v, f)| best_rule(cfg, leaf, &func.scope, f).map(|(l, _, _)| (m, v, l)))
        .collect()
}

/// One file's source lines, read on first use (most files have no finding).
struct SourceCache<'a> {
    root: &'a Path,
    path: &'a str,
    lines: Option<Vec<String>>,
}

impl<'a> SourceCache<'a> {
    fn new(root: &'a Path, path: &'a str) -> Self {
        SourceCache {
            root,
            path,
            lines: None,
        }
    }

    /// 1-based byte column of the name in `fn <name>` on the function's line, if found.
    fn col_of(&mut self, func: &brickie_facts::Function) -> Option<u32> {
        let (root, path) = (self.root, self.path);
        let lines = self.lines.get_or_insert_with(|| {
            std::fs::read_to_string(root.join(path))
                .map(|t| t.lines().map(str::to_string).collect())
                .unwrap_or_default()
        });
        let line = lines.get(func.line as usize - 1)?;
        line.find(&format!("fn {}", func.name))
            .map(|b| b as u32 + 4)
    }
}

/// The finding for one metric of a function whose leaf brick path is `leaf`, if the value
/// exceeds the limit the rules give it.
fn finding_for(
    cfg: &Config,
    leaf: &[String],
    func: &brickie_facts::Function,
    src: &mut SourceCache,
    (metric, value, limit_of): Metric,
) -> Option<Cx> {
    let (brick, parent) = leaf.split_last()?;
    let (limit, rule_index, rule) = best_rule(cfg, leaf, &func.scope, limit_of)?;
    if value <= limit {
        return None;
    }
    Some(Cx {
        brick: brick.clone(),
        level: level_name(parent),
        file: src.path.to_string(),
        qname: func.qname.clone(),
        metric,
        value,
        limit,
        line: func.line,
        col: src.col_of(func),
        name_len: func.name.len() as u32,
        rule_index,
        rule: rule.clone(),
    })
}

pub(super) fn complexity(
    cache: &Cache,
    cfg: &Config,
    res: &Resolved,
    root: &Path,
    out: &mut Vec<Box<dyn Finding>>,
) {
    for (path, f) in &cache.files {
        let mut src = SourceCache::new(root, path);
        for func in &f.functions {
            let Some(leaf) = func.module.as_deref().and_then(|m| res.path(m)) else {
                continue;
            };
            for metric in metrics(func) {
                let found = finding_for(cfg, leaf, func, &mut src, metric);
                out.extend(found.map(|c| Box::new(c) as Box<dyn Finding>));
            }
        }
    }
}

impl Finding for Cx {
    fn bricks(&self) -> Vec<String> {
        vec![self.brick.clone()]
    }
    fn brick_level(&self) -> Option<String> {
        Some(self.level.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn func(line: u32, name: &str) -> brickie_facts::Function {
        serde_json::from_value(serde_json::json!({
            "name": name, "qname": name, "module": null, "line": line, "end_line": line,
            "lines": 1, "cognitive": 0, "cyclomatic": 0, "nesting": 0, "paths": 1,
            "params": 0, "scope": "prod"
        }))
        .unwrap()
    }

    #[test]
    fn col_of_is_the_one_based_byte_column_of_the_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "// é\npub fn run() {}\n").unwrap();
        let mut src = SourceCache::new(dir.path(), "a.rs");
        assert_eq!(src.col_of(&func(2, "run")), Some(8));
        assert_eq!(src.col_of(&func(1, "run")), None);
        assert_eq!(src.col_of(&func(9, "run")), None);
        assert_eq!(
            SourceCache::new(dir.path(), "none.rs").col_of(&func(1, "run")),
            None
        );
    }
}
