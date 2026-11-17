//! Reading the design's file sheets for a level: the files that make it up (its own file and
//! what it includes), their rule and complexity rows, who shares a file, and what names a brick.

use super::{RuleRef, rule_key};
use brickie_web_api::DesignState;
use brickie_web_api::design::{ComplexitySpec, FileSheet, RuleSpec};

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

    /// The reference of the row read as a link.
    pub fn link_ref(&self) -> RuleRef {
        RuleRef::link(&self.file, &self.item)
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

/// The `[[rule]]` rows that apply at level `lv`.
pub fn level_rules(ch: &DesignState, lv: &str) -> Vec<Row<RuleSpec>> {
    rows(ch, lv, |s| &s.rules)
}

/// The `[[link]]` rows declared at level `lv`.
pub fn level_links(ch: &DesignState, lv: &str) -> Vec<Row<RuleSpec>> {
    rows(ch, lv, |s| &s.links)
}

/// The `[[complexity]]` rows that apply at level `lv`.
pub fn level_complexity(ch: &DesignState, lv: &str) -> Vec<Row<ComplexitySpec>> {
    rows(ch, lv, |s| &s.complexity)
}

/// The rule or link `r` names (`None`: no longer in the design).
pub fn rule_at<'a>(ch: &'a DesignState, r: &RuleRef) -> Option<&'a RuleSpec> {
    let s = ch.sheets.get(&r.file)?;
    let rules = if r.link { &s.links } else { &s.rules };
    rules.iter().find(|x| rule_key(x) == r.key)
}

/// How many levels read `file` (their own file or an include).
pub fn sharers(ch: &DesignState, file: &str) -> usize {
    ch.files
        .keys()
        .filter(|lv| level_files(ch, lv).iter().any(|f| f == file))
        .count()
}

/// What refers to brick `name` at level `lv`: rules and links naming it and the level's
/// exports.
pub fn references(ch: &DesignState, lv: &str, name: &str) -> Vec<String> {
    let mut out: Vec<String> = level_rules(ch, lv)
        .into_iter()
        .filter(|r| r.item.from == name || r.item.to == name)
        .map(|r| format!("rule {} \u{2192} {} ({})", r.item.from, r.item.to, r.file))
        .collect();
    out.extend(
        (level_links(ch, lv).into_iter())
            .filter(|r| r.item.from == name || super::link_brick(&r.item.to) == name)
            .map(|r| format!("link {} \u{2192} {} ({})", r.item.from, r.item.to, r.file)),
    );
    let own = ch.files.get(lv).and_then(|f| ch.sheets.get(f));
    if own.is_some_and(|s| s.exports.iter().any(|e| e == name)) {
        out.push("this level's exports".into());
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use brickie_web_api::design::By;
    use std::collections::BTreeMap;

    fn rule(from: &str, to: &str, kinds: &[&str]) -> RuleSpec {
        RuleSpec {
            from: from.into(),
            to: to.into(),
            kinds: kinds.iter().map(|k| k.to_string()).collect(),
            ..Default::default()
        }
    }

    pub fn change() -> DesignState {
        let own = FileSheet {
            rules: vec![rule("api", "ui", &["uses"])],
            links: vec![rule("ui", "api", &[])],
            exports: vec!["api".into()],
            include_files: vec!["shared.toml".into()],
            ..Default::default()
        };
        let shared = FileSheet {
            rules: vec![rule("*", "tag:io", &[])],
            ..Default::default()
        };
        DesignState {
            v: 1,
            published: 1,
            forked_from: None,
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
            link: false,
        };
        assert_eq!(rule_at(&ch, &gone), None);
        let l = level_links(&ch, "root/web")[0].link_ref();
        assert_eq!(rule_at(&ch, &l).map(|x| x.to.as_str()), Some("api"));
        assert_eq!(references(&ch, "root/web", "api").len(), 3);
    }
}
