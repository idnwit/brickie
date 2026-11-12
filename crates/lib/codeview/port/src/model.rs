//! The data a host gives the viewer: explorer trees, file texts, marks, bands and locations.

use serde::{Deserialize, Serialize};

/// A position in a file: 1-based line, 1-based byte column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

/// A range in a file, `start..end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}

/// A count shown next to a tree node (open errors and warnings, for example).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Badge {
    pub errors: u32,
    pub warns: u32,
}

/// What a node is, so the viewer can mark it (icon, colour). Presentation only; the host picks
/// the kinds that fit its trees.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// A plain group or an unknown node.
    #[default]
    Group,
    /// A leaf unit of the host's structure (a brick).
    Brick,
    /// A unit made of nested units.
    Composite,
    Crate,
    Folder,
    File,
}

/// One explorer node: a group (no `path`) or a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Stable id given by the host, the target of [`crate::ToViewer::Filter`].
    pub id: String,
    pub label: String,
    pub path: Option<String>,
    pub badge: Option<Badge>,
    pub children: Vec<Node>,
    #[serde(default)]
    pub kind: NodeKind,
    /// Short labels shown as chips; the viewer colours each by a hue derived from its text.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A named explorer tree ("Bricks", "Folders").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub name: String,
    pub roots: Vec<Node>,
}

/// A file's text. `stale`: changed since the host's data was computed (navigation may be off).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileText {
    pub path: String,
    pub text: String,
    pub lang: String,
    pub stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// A mark in the code: a finding or a highlight; `link` goes back to the host page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mark {
    pub span: Span,
    pub level: Level,
    pub message: String,
    pub link: Option<String>,
}

/// A navigation target; `label` says where it is from the host's point of view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub path: String,
    pub span: Span,
    pub label: String,
}

/// A band: a nested line-range annotation drawn as a bar in the gutter (a function and the
/// branches inside it, for example). Bands of one file may nest; `depth` is the gutter column
/// (0 = outermost). `heat` (0..=1, cold to hot) picks the colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Band {
    /// 1-based first and last line (inclusive).
    pub start_line: u32,
    pub end_line: u32,
    pub depth: u32,
    pub heat: f32,
    /// One line shown on hover.
    pub label: String,
    /// More lines shown on hover, under the label.
    pub detail: String,
}
