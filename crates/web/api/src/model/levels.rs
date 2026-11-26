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
    /// The `[[link]]`s declared at this level (view mode marks the planned ones).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<crate::LevelLink>,
    /// Enclosing-level `[[link]]`s crossing this composite's boundary (projected from ancestors).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundary_links: Vec<crate::BoundaryLink>,
    /// True when a `README.md` file sits alongside this level's `brickie.toml`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_readme: bool,
    /// Total production source lines under this level.
    #[serde(default)]
    pub code_lines: u32,
    /// Total test source lines under this level.
    #[serde(default)]
    pub test_lines: u32,
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

fn entry_hits(brick_level: Option<&str>, x: &str, path: &str, name: &str) -> bool {
    match brick_level {
        Some(lv) if !path.is_empty() => under(&format!("{lv}/{x}"), path),
        Some(_) => x == name,
        None => x == name || path == format!("root/{x}"),
    }
}

/// Whether `bricks` at `brick_level` names the brick at `(path, name)` (or a descendant of `path`).
///
/// When `brick_level` is `Some(lv)` and `path` is non-empty, matches `under("{lv}/{x}", path)` so
/// same-named bricks in different composites never collide. When `brick_level` is `None` (or `path`
/// is empty, e.g. synthetic graphs without paths), matches by name or `"root/{x}"`.
pub fn bricks_hit(bricks: &[String], brick_level: Option<&str>, path: &str, name: &str) -> bool {
    bricks
        .iter()
        .any(|x| entry_hits(brick_level, x, path, name))
}

/// Whether finding `d` names the brick at `(path, name)` (or a descendant of `path`).
pub(crate) fn diag_hits(d: &DiagView, path: &str, name: &str) -> bool {
    bricks_hit(&d.bricks, d.brick_level.as_deref(), path, name)
}

/// Whether finding `d` applies to a peer-group member: either it names the member (or a
/// descendant), or it is a `deps/cycle` finding on the enclosing group.
pub fn diag_hits_member(
    d: &DiagView,
    member_path: &str,
    member_name: &str,
    group_path: &str,
    group_name: &str,
) -> bool {
    diag_hits(d, member_path, member_name)
        || (d.code == "deps/cycle" && diag_hits(d, group_path, group_name))
}

/// The bricks of `g` a finding is about: each named child of the finding's level, lifted to
/// the brick of `g` that contains it (a deeper finding lands on its composite, a grouped member
/// finding lands on its enclosing group, an outside one on the faded neighbour). Findings without
/// a level (and bricks without a path) match by name.
pub fn diag_bricks(g: &GraphView, d: &DiagView) -> Vec<usize> {
    let lv = d.brick_level.as_deref();
    let mut out: Vec<usize> = Vec::new();
    for n in &d.bricks {
        let hit = g.bricks.iter().position(|b| {
            entry_hits(lv, n, &b.path, &b.name)
                || b.group_members
                    .iter()
                    .any(|m| entry_hits(lv, n, &m.path, &m.name))
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
            code: "deps/unlinked".into(),
            message: String::new(),
            span: None,
            spans: vec![],
            rule: None,
            help: None,
            notes: vec![],
            bricks: bricks.iter().map(|s| s.to_string()).collect(),
            brick_level: lv.map(str::to_string),
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

    #[test]
    fn member_findings_land_on_enclosing_group_once() {
        let g = GraphView {
            bricks: vec![
                BrickView {
                    name: "api".into(),
                    path: "root/api".into(),
                    ..Default::default()
                },
                BrickView {
                    name: "services".into(),
                    path: "root/services".into(),
                    is_group: true,
                    group_members: vec![
                        crate::model::GroupMemberView {
                            name: "auth".into(),
                            path: "root/auth".into(),
                            ..Default::default()
                        },
                        crate::model::GroupMemberView {
                            name: "billing".into(),
                            path: "root/billing".into(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let d = DiagView {
            id: "intra".into(),
            code: "deps/unlinked".into(),
            message: String::new(),
            span: None,
            spans: vec![],
            rule: None,
            help: None,
            notes: vec![],
            bricks: vec!["auth".into(), "billing".into()],
            brick_level: Some("root".into()),
            snippet: None,
        };
        assert_eq!(diag_bricks(&g, &d), [1]);
    }
}
