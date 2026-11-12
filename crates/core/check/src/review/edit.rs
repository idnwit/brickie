//! Accepting and unaccepting findings by editing the review file in place.

use super::review_path;
use anyhow::{Context, Result};
use std::path::Path;

/// Accept (`Some(reason)`) or unaccept (`None`) a finding in `brickie.review.toml`, creating
/// the file if needed and keeping its other content and comments.
pub fn set_review(config: &Path, id: &str, reason: Option<&str>) -> Result<()> {
    use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};
    let path = review_path(config);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc: DocumentMut = text
        .parse()
        .with_context(|| format!("parse {}", path.display()))?;
    if !doc.contains_key("accept") {
        doc["accept"] = Item::ArrayOfTables(ArrayOfTables::new());
    }
    let accepts = doc["accept"]
        .as_array_of_tables_mut()
        .context("`accept` in the review file is not an array of tables")?;
    let mut i = 0;
    while i < accepts.len() {
        if accepts.get(i).and_then(|t| t["id"].as_str()) == Some(id) {
            accepts.remove(i);
        } else {
            i += 1;
        }
    }
    if let Some(reason) = reason {
        let mut t = Table::new();
        t["id"] = value(id);
        t["reason"] = value(reason);
        accepts.push(t);
    }
    if accepts.is_empty() {
        doc.remove("accept");
    }
    std::fs::write(&path, doc.to_string()).with_context(|| format!("write {}", path.display()))
}
