//! Reading the design's file sheets for a level: the files that make it up (its own file and
//! what it includes), their rule and complexity rows, who shares a file, and what names a brick.

use super::{RuleRef, names, rule_key};
use brickie_web_api::design::{ComplexitySpec, FileSheet, RuleSpec};
use brickie_web_api::{BrickView, DesignState};

/// A row of a level's table: the physical file and the row.
#[derive(Debug, Clone, PartialEq)]
pub struct Row<T> {
    pub file: String,
    pub item: T,
}

impl Row<RuleSpec> {
    /// The rule's reference (file and identity).
    pub fn rule_ref(&self) -> RuleRef {
        RuleRef::of(&self.file, &self.item)
    }
}

/// The level's own file, then every file it includes (transitively, each once).
pub fn level_files(ch: &DesignState, lv: &str) -> Vec<String> {
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

fn rows<T: Clone>(ch: &DesignState, lv: &str, pick: impl Fn(&FileSheet) -> &Vec<T>) -> Vec<Row<T>> {
    let mut out = Vec::new();
    for f in level_files(ch, lv) {
        let Some(s) = ch.sheets.get(&f) else { continue };
        for item in pick(s) {
            out.push(Row {
                file: f.clone(),
                item: item.clone(),
            });
        }
    }
    out
}

/// The `[[deps.rule]]` rows that apply at level `lv`.
pub fn level_rules(ch: &DesignState, lv: &str) -> Vec<Row<RuleSpec>> {
    rows(ch, lv, |s| &s.rules)
}

/// The `[[complexity]]` rows that apply at level `lv`.
pub fn level_complexity(ch: &DesignState, lv: &str) -> Vec<Row<ComplexitySpec>> {
    rows(ch, lv, |s| &s.complexity)
}

/// The rule `r` names (`None`: no longer in the design).
pub fn rule_at<'a>(ch: &'a DesignState, r: &RuleRef) -> Option<&'a RuleSpec> {
    let rules = &ch.sheets.get(&r.file)?.rules;
    rules.iter().find(|x| rule_key(x) == r.key)
}

/// How many levels read `file` (their own file or an include).
pub fn sharers(ch: &DesignState, file: &str) -> usize {
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
pub fn references(ch: &DesignState, lv: &str, name: &str) -> Vec<String> {
    let mut out: Vec<String> = level_rules(ch, lv)
        .into_iter()
        .filter(|r| r.item.from == name || r.item.to == name)
        .map(|r| format!("rule {} \u{2192} {} ({})", r.item.from, r.item.to, r.file))
        .collect();
    let own = ch.files.get(lv).and_then(|f| ch.sheets.get(f));
    if own.is_some_and(|s| s.exports.iter().any(|e| e.brick == name)) {
        out.push("this level's exports".into());
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use brickie_web_api::design::{By, ExportSpec};
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

    pub fn change() -> DesignState {
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
        DesignState {
            v: 1,
            hash: String::new(),
            by: By::External,
            history: vec![],
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
        assert_eq!((rules.len(), rules[1].file.as_str()), (2, "shared.toml"));
        assert_eq!(sharers(&ch, "shared.toml"), 2);
        let r = rules[0].rule_ref();
        assert_eq!(rule_at(&ch, &r).map(|x| x.to.as_str()), Some("ui"));
        let gone = RuleRef {
            file: r.file.clone(),
            key: rule_key(&rule("api", "ui", &["implements"])),
        };
        assert_eq!(rule_at(&ch, &gone), None);
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
