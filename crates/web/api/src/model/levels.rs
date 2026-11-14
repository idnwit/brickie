//! The brick tree as levels: each level is a [`GraphView`] of its children and faded outside
//! neighbours; findings name children of their `brick_level` and land on the brick of the
//! level on screen that contains them.

use super::{DiagView, GraphView};
use serde::{Deserialize, Serialize};

/// One level of the brick tree: the children of a composite (or of the root), their faded
/// outside neighbours and the edges between them, already aggregated for this level.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LevelView {
    /// `root`, `root/web`.
    pub path: String,
    /// Config file of the level.
    pub file: String,
    /// Children visible from outside the composite (empty at the root).
    pub exports: Vec<String>,
    pub graph: GraphView,
    /// Edit mode: allow rules of this level that no measured edge uses yet (drawn dashed
    /// between the bricks their selectors name).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unused_links: Vec<crate::UnusedLink>,
}

impl LevelView {
    /// Last path component (`web`), `root` at the root.
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or("root")
    }
}

/// Index of the level `path` (falls back to the root, index 0).
pub fn level_index(levels: &[LevelView], path: &str) -> usize {
    let path = if path.is_empty() { "root" } else { path };
    levels.iter().position(|l| l.path == path).unwrap_or(0)
}

/// Whether `path` is `level` or below it.
pub fn under(path: &str, level: &str) -> bool {
    path == level || path.strip_prefix(level).is_some_and(|r| r.starts_with('/'))
}

/// The bricks of `g` a finding is about: each named child of the finding's level, lifted to
/// the brick of `g` that contains it (a deeper finding lands on its composite, an outside one on
/// the faded neighbour). Findings without a level (and bricks without a path) match by name.
pub fn diag_bricks(g: &GraphView, d: &DiagView) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for n in &d.bricks {
        let full = d.brick_level.as_ref().map(|lv| format!("{lv}/{n}"));
        let hit = g.bricks.iter().position(|b| match &full {
            _ if b.path.is_empty() => b.name == *n,
            Some(f) => under(f, &b.path),
            None => b.path == format!("root/{n}"),
        });
        if let Some(i) = hit
            && !out.contains(&i)
        {
            out.push(i);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BrickView;

    #[test]
    fn findings_land_on_the_brick_that_contains_them() {
        let b = |name: &str, path: &str| BrickView {
            name: name.into(),
            path: path.into(),
            ..Default::default()
        };
        // Level root/web: two children and an outside neighbour.
        let g = GraphView {
            bricks: vec![
                b("api", "root/web/api"),
                b("ui", "root/web/ui"),
                b("root/core", "root/core"),
            ],
            ..Default::default()
        };
        let d = |lv: Option<&str>, bricks: &[&str]| DiagView {
            id: "x".into(),
            level: "error".into(),
            code: "deps/deny".into(),
            message: String::new(),
            span: None,
            spans: vec![],
            rule: None,
            help: None,
            notes: vec![],
            bricks: bricks.iter().map(|s| s.to_string()).collect(),
            brick_level: lv.map(str::to_string),
            accepted: false,
            accepted_reason: None,
            snippet: None,
        };
        assert_eq!(
            diag_bricks(&g, &d(Some("root/web"), &["ui", "api"])),
            [1, 0]
        );
        // Deeper: lifted to the child; above: the frame itself is not a brick, core is a neighbour.
        assert_eq!(diag_bricks(&g, &d(Some("root/web/api"), &["h"])), [0]);
        assert_eq!(diag_bricks(&g, &d(Some("root"), &["web", "core"])), [2]);
        assert_eq!(level_index(&[], "root"), 0);
        assert!(under("root/web/api", "root/web") && !under("root/webx", "root/web"));
    }
}
