//! In-place refresh after an op: the levels of a `ChangeReply` replace the ones on the page,
//! and the canvas keeps its pan/zoom and the boxes where they were.

use crate::layout::Rect;
use crate::state::Vw;
use brickie_web_api::{BrickView, LevelView};
use std::collections::BTreeMap;

/// The canvas state carried over a patch: level, pan/zoom and box positions by brick name.
#[derive(Debug, Clone, PartialEq)]
pub struct Keep {
    pub level: String,
    pub view: Vw,
    pub pos: BTreeMap<String, (f64, f64)>,
}

/// `old` with each level of `new` replacing the one of the same path (new paths appended);
/// levels missing from `files` (the levels of the preview) are dropped, unless `files` is
/// empty (the preview did not load).
pub fn patch_levels(
    old: &[LevelView],
    new: Vec<LevelView>,
    files: &BTreeMap<String, String>,
) -> Vec<LevelView> {
    let mut out: Vec<LevelView> = old.to_vec();
    for lv in new {
        match out.iter_mut().find(|o| o.path == lv.path) {
            Some(o) => *o = lv,
            None => out.push(lv),
        }
    }
    if !files.is_empty() {
        out.retain(|l| l.path == "root" || files.contains_key(&l.path));
    }
    out
}

/// Positions of the boxes by brick name (faded neighbours excluded).
pub fn keep_positions(bricks: &[BrickView], pos: &[Rect]) -> BTreeMap<String, (f64, f64)> {
    bricks
        .iter()
        .zip(pos)
        .filter(|(b, _)| !b.ghost)
        .map(|(b, r)| (b.name.clone(), (r.x, r.y)))
        .collect()
}

/// Put the boxes of bricks that were on screen back where they were.
pub fn restore_positions(
    pos: &mut [Rect],
    bricks: &[BrickView],
    kept: &BTreeMap<String, (f64, f64)>,
) {
    for (r, b) in pos.iter_mut().zip(bricks) {
        if let Some(&(x, y)) = kept.get(&b.name).filter(|_| !b.ghost) {
            r.x = x;
            r.y = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(path: &str, file: &str) -> LevelView {
        LevelView {
            path: path.into(),
            file: file.into(),
            ..Default::default()
        }
    }

    #[test]
    fn reply_levels_replace_add_and_drop() {
        let old = vec![
            level("root", "a"),
            level("root/web", "b"),
            level("root/gone", "c"),
        ];
        let files = BTreeMap::from([
            ("root".to_string(), "brickie.toml".to_string()),
            ("root/web".to_string(), "web/brickie.toml".to_string()),
            ("root/new".to_string(), "new/brickie.toml".to_string()),
        ]);
        let out = patch_levels(
            &old,
            vec![level("root/web", "B"), level("root/new", "N")],
            &files,
        );
        let got: Vec<(&str, &str)> = out
            .iter()
            .map(|l| (l.path.as_str(), l.file.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![("root", "a"), ("root/web", "B"), ("root/new", "N")]
        );
        assert_eq!(patch_levels(&old, vec![], &BTreeMap::new()).len(), 3);
    }

    #[test]
    fn kept_boxes_stay_put() {
        let b = |n: &str| BrickView {
            name: n.into(),
            ..Default::default()
        };
        let r = |x: f64| Rect {
            x,
            y: 1.0,
            w: 10.0,
            h: 10.0,
        };
        let kept = keep_positions(&[b("a"), b("b")], &[r(5.0), r(50.0)]);
        let mut pos = vec![r(0.0), r(0.0), r(0.0)];
        restore_positions(&mut pos, &[b("b"), b("new"), b("a")], &kept);
        assert_eq!((pos[0].x, pos[1].x, pos[2].x), (50.0, 0.0, 5.0));
    }
}
