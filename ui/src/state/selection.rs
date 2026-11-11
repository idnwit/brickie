//! Selection (`S.sel`): a brick, a group of tag rows and bricks, or an edge, kept in the URL.

use crate::model::*;
use std::collections::BTreeSet;

/// `S.sel`.
#[derive(Debug, Clone, PartialEq)]
pub enum Sel {
    None,
    Brick(usize),
    /// Tag rows and individually added bricks.
    Group {
        tags: Vec<String>,
        bricks: Vec<usize>,
    },
    /// Index into [`GraphView::edges`].
    Edge(usize),
}

pub fn edge_key(g: &GraphView, e: usize) -> String {
    let x = &g.edges[e];
    format!(
        "{}>{}>{}",
        g.bricks[x.from].name, g.bricks[x.to].name, x.kind
    )
}

impl Sel {
    /// `applySel`: ids that do not exist are ignored.
    pub fn from_params(p: &Params, g: &GraphView) -> Sel {
        let idx = |n: &str| g.bricks.iter().position(|b| b.name == n);
        let bricks: Vec<usize> = p.bricks().iter().filter_map(|n| idx(n)).collect();
        let tags: Vec<String> = p
            .group
            .split(',')
            .filter(|t| {
                g.bricks
                    .iter()
                    .any(|b| b.tags.first().is_some_and(|x| x == t) || b.tag == *t)
            })
            .map(str::to_string)
            .collect();
        if !tags.is_empty() || bricks.len() > 1 {
            return Sel::Group { tags, bricks };
        }
        if let Some(&b) = bricks.first() {
            return Sel::Brick(b);
        }
        if let Some(e) = (0..g.edges.len()).find(|&e| edge_key(g, e) == p.edge) {
            return Sel::Edge(e);
        }
        Sel::None
    }

    /// Params for this selection (other params kept).
    pub fn apply(&self, p: &Params, g: &GraphView) -> Params {
        p.with(|p| {
            p.brick.clear();
            p.group.clear();
            p.edge.clear();
            p.item.clear();
            match self {
                Sel::None => {}
                Sel::Brick(b) => p.brick = g.bricks[*b].name.clone(),
                Sel::Group { tags, bricks } => {
                    p.group = tags.join(",");
                    p.brick = bricks
                        .iter()
                        .map(|b| g.bricks[*b].name.as_str())
                        .collect::<Vec<_>>()
                        .join(",");
                }
                Sel::Edge(e) => p.edge = edge_key(g, *e),
            }
        })
    }

    /// `selSet`: the selected bricks (an edge selects none; see [`Sel::scope`]).
    pub fn set(&self, g: &GraphView) -> BTreeSet<usize> {
        match self {
            Sel::None | Sel::Edge(_) => BTreeSet::new(),
            Sel::Brick(b) => [*b].into(),
            Sel::Group { tags, bricks } => g
                .bricks
                .iter()
                .enumerate()
                .filter(|(i, b)| bricks.contains(i) || tags.contains(&b.tag))
                .map(|(i, _)| i)
                .collect(),
        }
    }

    /// `scopeSet`: the bricks the lists are scoped to (`None` = everything).
    pub fn scope(&self, g: &GraphView) -> Option<BTreeSet<usize>> {
        match self {
            Sel::None => None,
            Sel::Edge(e) => Some([g.edges[*e].from, g.edges[*e].to].into()),
            _ => Some(self.set(g)),
        }
    }

    /// `scopeName`.
    pub fn name(&self, g: &GraphView) -> String {
        match self {
            Sel::None => "everything".into(),
            Sel::Brick(b) => format!("brick {}", g.bricks[*b].name),
            Sel::Edge(e) => {
                let x = &g.edges[*e];
                format!("edge {} -> {}", g.bricks[x.from].name, g.bricks[x.to].name)
            }
            Sel::Group { tags, bricks } => {
                let mut parts: Vec<String> = tags.iter().map(|t| format!("tag {t}")).collect();
                parts.extend(bricks.iter().map(|b| g.bricks[*b].name.clone()));
                format!("{} ({} bricks)", parts.join(" + "), self.set(g).len())
            }
        }
    }

    /// `onBrick`: click toggles a single brick.
    pub fn click(&self, b: usize) -> Sel {
        if *self == Sel::Brick(b) {
            Sel::None
        } else {
            Sel::Brick(b)
        }
    }

    /// `addBrick`: shift-click adds or removes a brick of the selection.
    pub fn add_brick(&self, b: usize) -> Sel {
        let (tags, mut bricks) = match self {
            Sel::Group { tags, bricks } => (tags.clone(), bricks.clone()),
            Sel::Brick(x) => (vec![], vec![*x]),
            _ => (vec![], vec![]),
        };
        match bricks.iter().position(|x| *x == b) {
            Some(i) => {
                bricks.remove(i);
            }
            None => bricks.push(b),
        }
        Sel::normalise(tags, bricks)
    }

    /// `selectGroup`: a tag row; `add` (shift) adds or removes it.
    pub fn select_tag(&self, tag: &str, add: bool) -> Sel {
        let (mut tags, bricks) = match (add, self) {
            (true, Sel::Group { tags, bricks }) => (tags.clone(), bricks.clone()),
            (true, Sel::Brick(x)) => (vec![], vec![*x]),
            _ => (vec![], vec![]),
        };
        match tags.iter().position(|t| t == tag) {
            Some(i) if add => {
                tags.remove(i);
            }
            _ => tags.push(tag.to_string()),
        }
        Sel::normalise(tags, bricks)
    }

    fn normalise(tags: Vec<String>, bricks: Vec<usize>) -> Sel {
        match (tags.is_empty(), bricks.len()) {
            (true, 0) => Sel::None,
            (true, 1) => Sel::Brick(bricks[0]),
            _ => Sel::Group { tags, bricks },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> GraphView {
        let b = |n: &str, t: &str, row| BrickView {
            name: n.into(),
            tag: t.into(),
            row,
            tags: vec![t.into()],
            members: vec![],
            worst_ratio: 0.0,
            hot: vec![],
        };
        GraphView {
            rows: vec!["core".into(), "app".into()],
            bricks: vec![b("a", "core", 0), b("b", "core", 0), b("c", "app", 1)],
            edges: vec![EdgeView {
                from: 2,
                to: 0,
                kind: "uses".into(),
                test: false,
                diags: vec![],
            }],
        }
    }

    #[test]
    fn selection_round_trip_and_group_ops() {
        let g = graph();
        let s = Sel::Brick(0).add_brick(1);
        assert_eq!(
            s,
            Sel::Group {
                tags: vec![],
                bricks: vec![0, 1]
            }
        );
        let p = s.apply(&Params::default(), &g);
        assert_eq!(Sel::from_params(&p, &g), s);
        assert_eq!(Sel::Brick(0).click(0), Sel::None);
        let t = Sel::None.select_tag("core", false);
        assert_eq!(t.set(&g), [0, 1].into());
        assert_eq!(t.select_tag("core", true), Sel::None);
        let e = Sel::Edge(0).apply(&Params::default(), &g);
        assert_eq!(Sel::from_params(&e, &g), Sel::Edge(0));
        assert_eq!(Sel::Edge(0).scope(&g), Some([2, 0].into()));
    }
}
