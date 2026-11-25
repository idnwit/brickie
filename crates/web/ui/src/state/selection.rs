//! Selection (`S.sel`): nothing, one brick or one link, kept in the URL.

use brickie_web_api::*;

/// `S.sel`.
#[derive(Debug, Clone, PartialEq)]
pub enum Sel {
    None,
    Brick(usize),
    /// Member `m` inside peer group brick `b` (`g.bricks[b].group_members[m]`).
    Member(usize, usize),
    /// Index into [`GraphView::edges`].
    Edge(usize),
}

pub(super) fn edge_key(g: &GraphView, e: usize) -> String {
    let x = &g.edges[e];
    format!(
        "{}>{}>{}",
        end_name(g, e, x.from),
        end_name(g, e, x.to),
        x.kind
    )
}

impl Sel {
    /// `applySel`: ids that do not exist are ignored; with several bricks in the URL the first
    /// known one wins.
    pub fn from_params(p: &Params, g: &GraphView) -> Sel {
        let idx = |n: &str| g.bricks.iter().position(|b| b.name == n);
        let mem = |n: &str| {
            g.bricks.iter().enumerate().find_map(|(bi, b)| {
                b.group_members
                    .iter()
                    .position(|m| m.name == n)
                    .map(|mi| (bi, mi))
            })
        };
        for n in p.bricks() {
            if let Some(b) = idx(n) {
                return Sel::Brick(b);
            }
            if let Some((b, m)) = mem(n) {
                return Sel::Member(b, m);
            }
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
            p.edge.clear();
            p.item.clear();
            match self {
                Sel::None => {}
                Sel::Brick(b) => p.brick = g.bricks[*b].name.clone(),
                Sel::Member(b, m) => p.brick = g.bricks[*b].group_members[*m].name.clone(),
                Sel::Edge(e) => p.edge = edge_key(g, *e),
            }
        })
    }

    /// The selected brick (a link selects none: the lists are scoped to the link itself).
    pub fn brick(&self) -> Option<usize> {
        match self {
            Sel::Brick(b) => Some(*b),
            _ => None,
        }
    }

    /// `scopeName`.
    pub fn name(&self, g: &GraphView) -> String {
        match self {
            Sel::None => "everything".into(),
            Sel::Brick(b) => {
                let br = &g.bricks[*b];
                if br.is_group {
                    format!("group {}", br.name)
                } else {
                    format!("brick {}", br.name)
                }
            }
            Sel::Member(b, m) => format!("brick {}", g.bricks[*b].group_members[*m].name),
            Sel::Edge(e) => {
                let x = &g.edges[*e];
                format!(
                    "edge {} -> {}",
                    end_name(g, *e, x.from),
                    end_name(g, *e, x.to)
                )
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

    /// Click toggles a member card inside group `b`.
    pub fn click_member(&self, b: usize, m: usize) -> Sel {
        if *self == Sel::Member(b, m) {
            Sel::None
        } else {
            Sel::Member(b, m)
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
            ..Default::default()
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
                ..Default::default()
            }],
        }
    }

    #[test]
    fn selection_round_trip_and_click() {
        let g = graph();
        let p = Sel::Brick(1).apply(&Params::default(), &g);
        assert_eq!(Sel::from_params(&p, &g), Sel::Brick(1));
        assert_eq!(Sel::Brick(0).click(0), Sel::None);
        assert_eq!(Sel::Brick(0).click(1), Sel::Brick(1));
        let e = Sel::Edge(0).apply(&Params::default(), &g);
        assert_eq!(Sel::from_params(&e, &g), Sel::Edge(0));
        // A link selects no brick: the lists are scoped to the link itself.
        assert_eq!(Sel::Edge(0).brick(), None);
        assert_eq!(Sel::Brick(2).brick(), Some(2));
    }
}
