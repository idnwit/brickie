//! Doctest uses: rust-analyzer does not index code blocks in doc comments, so the words of every
//! doctest are collected from the cached module and item docs, per crate. A public API item
//! counts as used by a doctest of its crate when its name (the last segment for an associated
//! item, since a method is often called on a value whose type is never written) appears in the
//! block. It is a word match, not name resolution.

use super::Site;
use brickie_facts::{Cache, Item};
use std::collections::{HashMap, HashSet};

/// One doctest: where its doc comment is and the identifiers it contains.
struct Block {
    file: String,
    line: u32,
    words: HashSet<String>,
}

/// Doctests by crate id.
pub(super) struct Doctests {
    by_crate: HashMap<String, Vec<Block>>,
    crate_of: HashMap<String, String>,
}

impl Doctests {
    pub fn new(cache: &Cache) -> Self {
        let crate_of: HashMap<String, String> = cache
            .modules
            .iter()
            .map(|m| (m.id.clone(), m.krate.clone()))
            .collect();
        let mut by_crate: HashMap<String, Vec<Block>> = HashMap::new();
        let mut add = |module: &str, file: &str, line: u32, doc: &str| {
            let Some(k) = crate_of.get(module) else {
                return;
            };
            for words in blocks(doc) {
                by_crate.entry(k.clone()).or_default().push(Block {
                    file: file.to_string(),
                    line,
                    words,
                });
            }
        };
        for m in &cache.modules {
            add(&m.id, &m.file, m.line, &m.doc);
        }
        for (file, fd) in &cache.files {
            for it in &fd.items {
                add(&it.module, file, it.line, &it.doc);
            }
        }
        Doctests { by_crate, crate_of }
    }

    /// Doctests of the item's crate that name it; only public API is visible to doctests.
    pub fn sites(&self, item: &Item) -> Vec<Site> {
        if !item.exported {
            return Vec::new();
        }
        let Some(blocks) = self
            .crate_of
            .get(&item.module)
            .and_then(|k| self.by_crate.get(k))
        else {
            return Vec::new();
        };
        let name = item.name.rsplit("::").next().unwrap_or_default();
        blocks
            .iter()
            .filter(|b| b.words.contains(name))
            .map(|b| Site {
                file: b.file.clone(),
                line: b.line,
                col: 1,
                end_col: 1,
                scope: "test".into(),
                kind: "doctest".into(),
                written: item.name.clone(),
                symbol: item.symbol.clone(),
            })
            .collect()
    }
}

/// Whether a fence info string marks a compiled doctest (rustdoc's default is Rust).
fn is_doctest(info: &str) -> bool {
    info.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .all(|t| {
            matches!(t, "rust" | "no_run" | "should_panic" | "test_harness")
                || t.starts_with("edition")
        })
}

/// The identifier sets of the doctest code blocks of a markdown doc.
pub(super) fn blocks(doc: &str) -> Vec<HashSet<String>> {
    let mut out = Vec::new();
    let mut open: Option<(String, bool, HashSet<String>)> = None;
    for line in doc.lines() {
        let t = line.trim_start();
        let fence = ["```", "~~~"].into_iter().find(|f| t.starts_with(f));
        match (&mut open, fence) {
            (None, Some(f)) => open = Some((f.into(), is_doctest(&t[3..]), HashSet::new())),
            (Some((f, test, words)), Some(g)) if f == g => {
                if *test {
                    out.push(std::mem::take(words));
                }
                open = None;
            }
            (Some((_, true, words)), _) => {
                words.extend(
                    t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .filter(|w| !w.is_empty())
                        .map(String::from),
                );
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::blocks;

    #[test]
    fn only_compiled_fences_count() {
        let doc = "x\n```\nuse a::Foo;\n```\n```text\nBar\n```\n```ignore\nBaz\n```\n```no_run\nQux::new()\n```";
        let b = blocks(doc);
        assert_eq!(b.len(), 2);
        assert!(b[0].contains("Foo"));
        assert!(b[1].contains("Qux") && b[1].contains("new"));
    }
}
