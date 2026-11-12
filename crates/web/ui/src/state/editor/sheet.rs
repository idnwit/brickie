//! Reading the change's file sheets for a level: the files that make it up (its own file and
//! what it includes), their rule and complexity rows, who shares a file, and what names a brick.

use super::{RuleRef, names};
use brickie_web_api::change::{ComplexitySpec, RuleSpec};
use brickie_web_api::{BrickView, ChangeView};

/// A row of a level's table: the physical file, the index in it, the row.
#[derive(Debug, Clone, PartialEq)]
pub struct Row<T> {
    pub file: String,
    pub index: usize,
    pub item: T,
}

/// The level's own file, then every file it includes (transitively, each once).
pub fn level_files(ch: &ChangeView, lv: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut todo: Vec<String> = ch.files.get(lv).cloned().into_iter().collect();
    while let Some(f) = todo.pop() {
        if out.contains(&f) {
            continue;
        }
        if let Some(s) = ch.sheets.get(&f) {
            todo.extend(s.include_files.iter().rev().cloned());
        }
        out.push(f);
    }
    out
}

fn rows<T: Clone>(
    ch: &ChangeView,
    lv: &str,
    pick: impl Fn(&brickie_web_api::change::FileSheet) -> &Vec<T>,
) -> Vec<Row<T>> {
    let mut out = Vec::new();
    for f in level_files(ch, lv) {
        let Some(s) = ch.sheets.get(&f) else { continue };
        for (index, item) in pick(s).iter().enumerate() {
            out.push(Row {
                file: f.clone(),
                index,
                item: item.clone(),
            });
        }
    }
    out
}

/// The `[[deps.rule]]` rows that apply at level `lv`.
pub fn level_rules(ch: &ChangeView, lv: &str) -> Vec<Row<RuleSpec>> {
    rows(ch, lv, |s| &s.rules)
}

/// The `[[complexity]]` rows that apply at level `lv`.
pub fn level_complexity(ch: &ChangeView, lv: &str) -> Vec<Row<ComplexitySpec>> {
    rows(ch, lv, |s| &s.complexity)
}

/// The rule at `r`.
pub fn rule_at<'a>(ch: &'a ChangeView, r: &RuleRef) -> Option<&'a RuleSpec> {
    ch.sheets.get(&r.file)?.rules.get(r.index)
}

/// The last rule of `file` (the one an `add_rule` just appended).
pub fn last_rule(ch: &ChangeView, file: &str) -> Option<RuleRef> {
    let n = ch.sheets.get(file)?.rules.len();
    (n > 0).then(|| RuleRef {
        file: file.to_string(),
        index: n - 1,
    })
}

/// How many levels read `file` (their own file or an include).
pub fn sharers(ch: &ChangeView, file: &str) -> usize {
    ch.files
        .keys()
        .filter(|lv| level_files(ch, lv).iter().any(|f| f == file))
        .count()
}

/// Rules of the level naming `from -> to` (by name, tag or `*`), and covering `kind`.
pub fn deciding(
    rules: &[Row<RuleSpec>],
    from: &BrickView,
    to: &BrickView,
    kind: &str,
) -> Vec<Row<RuleSpec>> {
    rules
        .iter()
        .filter(|r| names(&r.item.from, from) && names(&r.item.to, to))
        .filter(|r| r.item.kinds.is_empty() || r.item.kinds.iter().any(|k| k == kind))
        .cloned()
        .collect()
}

/// What refers to brick `name` at level `lv`: rules naming it and the level's exports.
pub fn references(ch: &ChangeView, lv: &str, name: &str) -> Vec<String> {
    let mut out: Vec<String> = level_rules(ch, lv)
        .into_iter()
        .filter(|r| r.item.from == name || r.item.to == name)
        .map(|r| {
            format!(
                "rule {} \u{2192} {} ({}#{})",
                r.item.from, r.item.to, r.file, r.index
            )
        })
        .collect();
    let own = ch.files.get(lv).and_then(|f| ch.sheets.get(f));
    if own.is_some_and(|s| s.exports.iter().any(|e| e.brick == name)) {
        out.push("this level's exports".into());
    }
    out
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use brickie_web_api::change::{ExportSpec, FileSheet, Status};
    use std::collections::BTreeMap;

    fn rule(from: &str, to: &str, kinds: &[&str]) -> RuleSpec {
        RuleSpec {
            from: from.into(),
            to: to.into(),
            kinds: kinds.iter().map(|k| k.to_string()).collect(),
            action: "allow".into(),
            ..Default::default()
        }
    }

    pub fn change() -> ChangeView {
        let own = FileSheet {
            rules: vec![rule("api", "ui", &["uses"])],
            exports: vec![ExportSpec {
                brick: "api".into(),
                scope: None,
            }],
            include_files: vec!["shared.toml".into()],
            ..Default::default()
        };
        let shared = FileSheet {
            rules: vec![rule("*", "tag:io", &[])],
            ..Default::default()
        };
        ChangeView {
            status: Status {
                intent: String::new(),
                head: 0,
                proposed: 0,
                accepted: 0,
                rejected: 0,
                conflicts: 0,
                cache_age_secs: None,
            },
            log: vec![],
            diff: vec![],
            todo: vec![],
            layout: BTreeMap::new(),
            files: BTreeMap::from([
                ("root".to_string(), "brickie.toml".to_string()),
                ("root/web".to_string(), "web/brickie.toml".to_string()),
            ]),
            levels: BTreeMap::new(),
            planned_edges: vec![],
            sheets: BTreeMap::from([
                ("web/brickie.toml".to_string(), own),
                ("shared.toml".to_string(), shared),
                (
                    "brickie.toml".to_string(),
                    FileSheet {
                        include_files: vec!["shared.toml".into()],
                        ..Default::default()
                    },
                ),
            ]),
            error: None,
        }
    }

    #[test]
    fn a_level_reads_its_file_and_includes() {
        let ch = change();
        assert_eq!(
            level_files(&ch, "root/web"),
            vec!["web/brickie.toml", "shared.toml"]
        );
        let rules = level_rules(&ch, "root/web");
        assert_eq!(
            (rules.len(), rules[1].file.as_str(), rules[1].index),
            (2, "shared.toml", 0)
        );
        assert_eq!(sharers(&ch, "shared.toml"), 2);
        assert_eq!(
            last_rule(&ch, "web/brickie.toml"),
            Some(RuleRef {
                file: "web/brickie.toml".into(),
                index: 0
            })
        );
        assert_eq!(references(&ch, "root/web", "api").len(), 2);
    }

    #[test]
    fn deciding_rules_match_selectors_and_kinds() {
        let ch = change();
        let rules = level_rules(&ch, "root/web");
        let b = |n: &str, tags: &[&str]| BrickView {
            name: n.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        };
        assert_eq!(
            deciding(&rules, &b("api", &[]), &b("ui", &["io"]), "uses").len(),
            2
        );
        assert_eq!(
            deciding(&rules, &b("api", &[]), &b("ui", &[]), "implements").len(),
            0
        );
    }
}
