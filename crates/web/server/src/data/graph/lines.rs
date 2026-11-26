//! Per-node code and test line counts aggregated from the measure cache.

use brickie_check::check::Loaded;
use brickie_check::config::{Config, level_name};
use brickie_facts::FileData;
use std::collections::BTreeMap;

/// `(code_lines, test_lines)` keyed by node level path (`root`, `root/web/api`); a composite
/// collects its whole subtree and a peer group collects its member bricks.
pub fn line_counts(l: &Loaded) -> BTreeMap<String, (u32, u32)> {
    let mut out: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for f in l.cache.files.values() {
        let Some(leaf) = file_leaf(l, f) else {
            continue;
        };
        let (code, test) = file_lines(f);
        add_to_nodes(&mut out, &l.cfg, leaf, code, test);
    }
    out
}

fn file_leaf<'a>(l: &'a Loaded, f: &FileData) -> Option<&'a [String]> {
    if let Some(m) = f.module.as_deref()
        && let Some(leaf) = l.resolved.path(m)
    {
        return Some(leaf);
    }
    f.functions
        .iter()
        .filter_map(|func| func.module.as_deref().and_then(|m| l.resolved.path(m)))
        .next()
}

/// Split one file's line count into `(code_lines, test_lines)`.
fn file_lines(f: &FileData) -> (u32, u32) {
    if f.scope == "test" {
        return (0, f.lines);
    }
    let test = merged_test_fn_lines(f).min(f.lines);
    (f.lines.saturating_sub(test), test)
}

/// Union of `[line, end_line]` intervals of test-scoped functions in `f` (avoids double-counting
/// nested helper functions inside test blocks).
fn merged_test_fn_lines(f: &FileData) -> u32 {
    let mut spans: Vec<(u32, u32)> = f
        .functions
        .iter()
        .filter(|func| func.scope == "test")
        .map(|func| (func.line, func.end_line.max(func.line)))
        .collect();
    if spans.is_empty() {
        return 0;
    }
    spans.sort_unstable();
    let mut total = 0u32;
    let (mut cur_s, mut cur_e) = spans[0];
    for (s, e) in spans.into_iter().skip(1) {
        if s <= cur_e.saturating_add(1) {
            cur_e = cur_e.max(e);
        } else {
            total = total.saturating_add(cur_e - cur_s + 1);
            (cur_s, cur_e) = (s, e);
        }
    }
    total.saturating_add(cur_e - cur_s + 1)
}

fn bump(out: &mut BTreeMap<String, (u32, u32)>, key: String, code: u32, test: u32) {
    let e = out.entry(key).or_default();
    e.0 = e.0.saturating_add(code);
    e.1 = e.1.saturating_add(test);
}

fn add_to_nodes(
    out: &mut BTreeMap<String, (u32, u32)>,
    cfg: &Config,
    leaf: &[String],
    code: u32,
    test: u32,
) {
    bump(out, "root".into(), code, test);
    for n in 1..=leaf.len() {
        let prefix = &leaf[..n];
        bump(out, level_name(prefix), code, test);
        let parent = &leaf[..n - 1];
        if let Some(lv) = cfg.node(parent)
            && let Some(b) = lv.brick(&leaf[n - 1])
            && let Some(g) = &b.group
        {
            let mut gpath = parent.to_vec();
            gpath.push(g.clone());
            bump(out, level_name(&gpath), code, test);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_facts::Function;

    fn test_fn(line: u32, end_line: u32) -> Function {
        Function {
            name: "t".into(),
            qname: "t".into(),
            module: None,
            line,
            end_line,
            lines: end_line - line + 1,
            cognitive: 0,
            cyclomatic: 1,
            nesting: 0,
            paths: 1,
            params: 0,
            scope: "test".into(),
        }
    }

    #[test]
    fn splits_prod_and_test_lines_without_double_counting_nested_fns() {
        let prod = FileData {
            sha256: String::new(),
            compiled: true,
            size: 0,
            lines: 100,
            scope: "prod".into(),
            module: None,
            functions: vec![test_fn(60, 80), test_fn(65, 70), test_fn(82, 90)],
            edges: vec![],
            items: vec![],
        };
        assert_eq!(file_lines(&prod), (70, 30));

        let integ = FileData {
            scope: "test".into(),
            lines: 45,
            functions: vec![],
            ..prod
        };
        assert_eq!(file_lines(&integ), (0, 45));
    }
}
