//! The design's config files as [`FileSheet`]s: each file's own rules, links,
//! complexity rules, exports and includes, in file order (the viewer's tables).

use crate::design::Design;
use crate::paths::join_rel;
use anyhow::Result;
use brickie_model::design::{ComplexitySpec, FileSheet, RuleSpec};
use std::collections::BTreeMap;
use toml::Value;

/// The entries of an array as `T`; one that does not deserialize is a default row, so rows keep
/// their place (the loader reports the bad entry).
fn list<T: serde::de::DeserializeOwned + Default>(v: Option<&Value>) -> Vec<T> {
    let items = v.and_then(Value::as_array).cloned().unwrap_or_default();
    items
        .into_iter()
        .map(|i| i.try_into().unwrap_or_default())
        .collect()
}

/// The sheet of the config file `file` with text `src` (`None` when it does not parse).
pub fn sheet(file: &str, src: &str) -> Option<FileSheet> {
    let doc: toml::Table = src.parse().ok()?;
    let include: Vec<String> = list(doc.get("include"));
    Some(FileSheet {
        rules: list::<RuleSpec>(doc.get("rule")),
        links: list::<RuleSpec>(doc.get("link")),
        exports: list(doc.get("exports")),
        complexity: list::<ComplexitySpec>(doc.get("complexity")),
        include_files: include.iter().map(|i| join_rel(file, i)).collect(),
        include,
    })
}

impl Design {
    /// Every config file of the design on disk that parses, by path relative to the root
    /// config's directory.
    pub fn sheets(&self) -> Result<BTreeMap<String, FileSheet>> {
        Ok(self
            .files()?
            .iter()
            .filter_map(|(f, b)| Some((f.clone(), sheet(f, std::str::from_utf8(b).ok()?)?)))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_reads_as_its_own_rows() {
        let src = r#"
include = ["../shared.toml"]
exports = ["api", "ui"]
[[rule]]
from = "api"
to = "ui"
kinds = ["uses"]
reason = "ui may use the api"
[[complexity]]
cognitive = 10
"#;
        let s = sheet("web/brickie.toml", src).unwrap();
        assert_eq!((s.rules.len(), s.rules[0].to.as_str()), (1, "ui"));
        assert_eq!(s.complexity[0].cognitive, Some(10));
        assert_eq!(s.exports, ["api", "ui"]);
        assert_eq!(s.include_files, vec!["shared.toml"]);
    }
}
