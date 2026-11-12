//! Lifting a module edge to the lowest common ancestor of its two leaf paths, and the
//! visibility of the target through the exports of the composites it sits in.

use crate::config::{Brick, Config};

/// A module edge lifted to the level where its ends are distinct siblings.
pub struct Lifted<'a> {
    /// The composite (or the root) whose children `from` and `to` are.
    pub level: &'a Config,
    /// Depth of `from` / `to` in the paths.
    pub k: usize,
    pub from: &'a Brick,
    pub to: &'a Brick,
}

/// `None` when the paths are equal (an edge inside one leaf) or the tree has no such node.
pub fn lift<'a>(cfg: &'a Config, p1: &[String], p2: &[String]) -> Option<Lifted<'a>> {
    let k = p1.iter().zip(p2).take_while(|(a, b)| a == b).count();
    let (x, y) = (p1.get(k)?, p2.get(k)?);
    let level = cfg.node(&p1[..k])?;
    Some(Lifted {
        level,
        k,
        from: level.brick(x)?,
        to: level.brick(y)?,
    })
}

/// The first composite on the target path below the common ancestor that does not export the
/// next brick for edges of `scope`: (composite level, hidden child name).
pub fn hidden<'a>(
    cfg: &'a Config,
    p2: &'a [String],
    k: usize,
    scope: &str,
) -> Option<(&'a Config, &'a str)> {
    (k + 1..p2.len()).find_map(|j| {
        let composite = cfg.node(&p2[..j])?;
        let child = p2[j].as_str();
        (!composite.exports_to(child, scope)).then_some((composite, child))
    })
}
