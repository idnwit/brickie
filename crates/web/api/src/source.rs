//! Data for the source popup page, handed from the server to the UI's renderer.

use brickie_model::FnTree;

#[derive(Debug, Clone, PartialEq)]
pub struct SourceLine {
    pub n: u32,
    /// Line text split around the highlighted span; for a whole-line highlight `hit` is empty
    /// and `line_hl` is set.
    pub before: String,
    pub hit: String,
    pub after: String,
    pub line_hl: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourcePage {
    pub file: String,
    pub lines: Vec<SourceLine>,
    /// Decision tree of the function around the target line.
    pub tree: Option<FnTree>,
}
