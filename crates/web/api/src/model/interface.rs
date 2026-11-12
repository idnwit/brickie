//! A brick's declared interface and the reference sites behind an edge (`Interface`,
//! `EdgeUses` endpoints).

use serde::{Deserialize, Serialize};

/// Body of `POST /api/interface`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceRequest {
    /// Brick path (`root/web/api`).
    pub brick: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceView {
    pub brick: String,
    /// Most used first, unused last.
    pub items: Vec<IfaceItemView>,
}

impl InterfaceView {
    pub fn unused(&self) -> usize {
        self.items.iter().filter(|i| i.uses.is_empty()).count()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfaceItemView {
    /// Defining leaf brick path.
    pub leaf: String,
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub doc: String,
    pub file: String,
    pub line: u32,
    /// By using brick at the interface's level; empty when unused.
    pub uses: Vec<UseGroupView>,
}

impl IfaceItemView {
    pub fn use_count(&self) -> usize {
        self.uses.iter().map(|g| g.sites.len()).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseGroupView {
    /// Using brick path (`root/web/server`), `<brick> (tests)` or `(unmapped)`.
    pub brick: String,
    pub sites: Vec<SiteView>,
}

/// One reference site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteView {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_col: u32,
    /// `prod` or `test`.
    pub scope: String,
    /// `uses`, `implements` or `reexports`.
    pub kind: String,
    pub written: String,
    pub symbol: String,
}

/// Body of `POST /api/edge-uses`: brick paths of the edge's ends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeUsesRequest {
    pub from: String,
    pub to: String,
}

/// One used item of the edge's target and every site that uses it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeItemView {
    pub module: String,
    pub name: String,
    /// `""` for an item that is not part of any interface (private).
    pub kind: String,
    pub signature: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub sites: Vec<SiteView>,
}
