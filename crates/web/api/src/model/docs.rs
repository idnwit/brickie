//! The documentation page of a brick (`GET /docs/<brick path>`): its place in the brick tree,
//! its children and the items it declares, with full docs and real use sites.

use super::{SiteView, encode};
use serde::{Deserialize, Serialize};

/// The docs page of the brick at `brick` (`root` for the tree root).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsPage {
    /// Level path (`root/core/model`, or `root`).
    pub brick: String,
    pub desc: Option<String>,
    /// The inner doc comment (`//!`) of the brick's root module (markdown), `""` when it has
    /// none or no single root module (a composite of several crates).
    pub doc: String,
    /// The whole brick tree in pre-order, for the sidebar.
    pub tree: Vec<DocsNode>,
    /// Direct children of a composite (the top bricks for `root`).
    pub children: Vec<DocsNode>,
    /// The declared interface: a leaf's items, or a composite's exported children's items.
    pub items: Vec<DocsItem>,
}

/// One brick of the tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsNode {
    /// Level path (`root/core/model`).
    pub path: String,
    pub name: String,
    /// 1 for a top brick.
    pub depth: u32,
    pub desc: Option<String>,
    /// Exported by its parent composite (always true at the top level).
    pub exported: bool,
}

/// One declared item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsItem {
    /// Defining leaf brick path.
    pub leaf: String,
    pub kind: String,
    /// `Type::name` for associated items.
    pub name: String,
    pub signature: String,
    /// The whole doc comment (markdown).
    pub doc: String,
    pub file: String,
    pub line: u32,
    pub uses: Vec<DocsUse>,
}

/// The uses of an item by one brick: the count and a few example sites.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsUse {
    pub brick: String,
    pub count: usize,
    pub examples: Vec<DocsExample>,
}

/// A use site with its source line (empty when the file cannot be read).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsExample {
    pub site: SiteView,
    pub code: String,
}

/// The docs page of the brick `node` (its level path, for example `root/web/server`).
pub fn docs_href(node: &str) -> String {
    let rest = node.strip_prefix("root").unwrap_or(node).trim_matches('/');
    let rest: Vec<String> = rest
        .split('/')
        .filter(|s| !s.is_empty())
        .map(encode)
        .collect();
    format!("{}{}", crate::DOCS_PATH, rest.join("/"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn hrefs() {
        assert_eq!(super::docs_href("root"), "/docs/");
        assert_eq!(super::docs_href("root/core/model"), "/docs/core/model");
    }
}
