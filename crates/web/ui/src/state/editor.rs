//! The design editor's state: its signals ([`Edit`], shared by every level) and the pure
//! helpers the components use to build ops, place bricks and hit-test drops. No DOM access.

use crate::layout::Rect;
use brickie_web_api::change::{BrickKind, BrickSpec, LayoutHint, LinkOption, Op, Plan, RuleSpec};
use brickie_web_api::{BrickView, ChangeView};
use std::collections::BTreeMap;
use sycamore::prelude::*;

/// A brick being added: its kind and canvas (world) position; the name is asked inline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draft {
    pub kind: BrickKind,
    pub x: f64,
    pub y: f64,
}

/// A link picked from the inspector: drop it on a sibling to add the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkPick {
    pub from: String,
    pub kind: String,
    pub scope: String,
    /// `allow` or `deny`: the rule to add.
    pub action: String,
}

/// An edit waiting for the user's confirmation (a rule needs a reason).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    Rule { pick: LinkPick, to: String },
    Move { from: String, into: String },
}

/// The modal dialog of the change bar.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Dialog {
    #[default]
    None,
    Open,
    /// The publish plan (`None` while it loads).
    Publish(Option<Plan>),
    /// Published: offer to measure now (the cache predates the new bricks).
    Published,
    Discard,
}

/// The editor's signals (all `Copy`).
#[derive(Clone, Copy)]
pub struct Edit {
    /// The palette kind armed for the next placement.
    pub tool: Signal<Option<BrickKind>>,
    /// A palette kind is being dragged onto the stage.
    pub carry: Signal<bool>,
    pub draft: Signal<Option<Draft>>,
    /// Link options of the selected brick (fetched on selection).
    pub links: Signal<Vec<LinkOption>>,
    pub link: Signal<Option<LinkPick>>,
    pub pending: Signal<Option<Pending>>,
    pub dialog: Signal<Dialog>,
    /// Right panel tab: `inspect` or `log`.
    pub tab: Signal<&'static str>,
    /// The last error of a change request (empty: none).
    pub msg: Signal<String>,
    pub busy: Signal<bool>,
}

impl Edit {
    pub(super) fn new() -> Edit {
        Edit {
            tool: create_signal(None),
            carry: create_signal(false),
            draft: create_signal(None),
            links: create_signal(Vec::new()),
            link: create_signal(None),
            pending: create_signal(None),
            dialog: create_signal(Dialog::None),
            tab: create_signal("inspect"),
            msg: create_signal(String::new()),
            busy: create_signal(false),
        }
    }

    /// Drop the armed tool or link and any open form; false when nothing was armed or open.
    pub fn cancel(&self) -> bool {
        let armed = self.tool.get().is_some() || self.link.with(|l| l.is_some());
        let open = self.draft.get().is_some() || self.pending.with(|p| p.is_some());
        self.tool.set(None);
        self.link.set(None);
        self.draft.set(None);
        self.pending.set(None);
        armed || open
    }
}

/// The brick path from the root (without `root`) of brick `name` at level `lv`.
pub fn brick_path(lv: &str, name: &str) -> Vec<String> {
    lv.split('/')
        .skip(1)
        .map(str::to_string)
        .chain(std::iter::once(name.to_string()))
        .collect()
}

/// The layout key (level path) of brick `name` at level `lv`.
pub fn layout_key(lv: &str, name: &str) -> String {
    format!("{lv}/{name}")
}

/// The palette's `add_brick` (a module names its crate in `members`, `crate::module`).
pub fn add_brick_op(file: &str, kind: BrickKind, name: &str, members: &str) -> Op {
    Op::AddBrick {
        file: file.to_string(),
        brick: BrickSpec {
            name: name.trim().to_string(),
            kind: Some(kind),
            members: split_list(members),
            ..Default::default()
        },
    }
}

/// The rule a dropped link adds.
pub fn rule_op(file: &str, pick: &LinkPick, to: &str, reason: &str) -> Op {
    let reason = reason.trim();
    Op::AddRule {
        file: file.to_string(),
        rule: RuleSpec {
            from: pick.from.clone(),
            to: to.to_string(),
            kinds: vec![pick.kind.clone()],
            scope: Some(pick.scope.clone()),
            action: pick.action.clone(),
            reason: (!reason.is_empty()).then(|| reason.to_string()),
            help: None,
        },
    }
}

/// `a, b ,c` -> `[a, b, c]`.
pub fn split_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// The distinct (kind, scope) pairs of the link options, in order.
pub fn link_kinds(opts: &[LinkOption]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for o in opts {
        let k = (o.kind.clone(), o.scope.clone());
        if !out.contains(&k) {
            out.push(k);
        }
    }
    out
}

/// What the rules decide now for `pick` dropped on `to` (`None`: not a sibling).
pub fn decided<'a>(opts: &'a [LinkOption], pick: &LinkPick, to: &str) -> Option<&'a LinkOption> {
    opts.iter()
        .find(|o| o.to == to && o.kind == pick.kind && o.scope == pick.scope)
}

/// Brick boxes moved to their stored canvas positions (level `lv`).
pub fn apply_hints(
    pos: &mut [Rect],
    bricks: &[BrickView],
    hints: &BTreeMap<String, LayoutHint>,
    lv: &str,
) {
    for (r, b) in pos.iter_mut().zip(bricks) {
        if let Some(h) = hints.get(&layout_key(lv, &b.name)).filter(|_| !b.ghost) {
            r.x = h.x;
            r.y = h.y;
        }
    }
}

/// The brick (not a faded neighbour, not `skip`) whose box holds the world point.
pub fn hit(
    pos: &[Rect],
    bricks: &[BrickView],
    (x, y): (f64, f64),
    skip: Option<usize>,
) -> Option<usize> {
    (0..pos.len().min(bricks.len())).find(|&i| {
        let r = &pos[i];
        Some(i) != skip
            && !bricks[i].ghost
            && x >= r.x
            && x <= r.x + r.w
            && y >= r.y
            && y <= r.y + r.h
    })
}

/// The editor is on: the preview is shown and a change is open.
pub(super) fn editing(change: Option<&ChangeView>, p: &brickie_web_api::Params) -> bool {
    change.is_some() && p.editing()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brick(name: &str, ghost: bool) -> BrickView {
        BrickView {
            name: name.into(),
            ghost,
            ..Default::default()
        }
    }

    fn rect(x: f64, y: f64) -> Rect {
        Rect {
            x,
            y,
            w: 100.0,
            h: 50.0,
        }
    }

    #[test]
    fn paths_and_keys() {
        assert_eq!(brick_path("root", "cli"), vec!["cli"]);
        assert_eq!(brick_path("root/web", "api"), vec!["web", "api"]);
        assert_eq!(layout_key("root/web", "api"), "root/web/api");
        assert_eq!(split_list(" a, ,b "), vec!["a", "b"]);
    }

    #[test]
    fn ops_from_the_palette_and_links() {
        let op = add_brick_op("brickie.toml", BrickKind::Crate, " cache ", "");
        assert_eq!(serde_json::to_value(&op).unwrap()["brick"]["name"], "cache");
        let pick = LinkPick {
            from: "a".into(),
            kind: "uses".into(),
            scope: "prod".into(),
            action: "allow".into(),
        };
        let Op::AddRule { rule, .. } = rule_op("x.toml", &pick, "b", " because ") else {
            panic!("add_rule");
        };
        assert_eq!(
            (rule.to.as_str(), rule.reason.as_deref()),
            ("b", Some("because"))
        );
        assert_eq!(rule.kinds, vec!["uses"]);
    }

    #[test]
    fn link_kinds_dedupe_and_decide() {
        let o = |to: &str, kind: &str, action: &str| LinkOption {
            to: to.into(),
            kind: kind.into(),
            scope: "prod".into(),
            action: action.into(),
            rule: None,
        };
        let opts = vec![
            o("b", "uses", "deny"),
            o("c", "uses", "allow"),
            o("b", "implements", "deny"),
        ];
        assert_eq!(link_kinds(&opts).len(), 2);
        let pick = LinkPick {
            from: "a".into(),
            kind: "uses".into(),
            scope: "prod".into(),
            action: "allow".into(),
        };
        assert_eq!(
            decided(&opts, &pick, "c").map(|o| o.action.as_str()),
            Some("allow")
        );
        assert!(decided(&opts, &pick, "zz").is_none());
    }

    #[test]
    fn hints_move_boxes_and_hits_skip_ghosts() {
        let bricks = vec![brick("a", false), brick("out", true)];
        let mut pos = vec![rect(0.0, 0.0), rect(200.0, 0.0)];
        let hints = BTreeMap::from([
            ("root/a".to_string(), LayoutHint { x: 300.0, y: 40.0 }),
            ("root/out".to_string(), LayoutHint { x: 9.0, y: 9.0 }),
        ]);
        apply_hints(&mut pos, &bricks, &hints, "root");
        assert_eq!((pos[0].x, pos[0].y, pos[1].x), (300.0, 40.0, 200.0));
        assert_eq!(hit(&pos, &bricks, (310.0, 50.0), None), Some(0));
        assert_eq!(hit(&pos, &bricks, (310.0, 50.0), Some(0)), None);
        assert_eq!(hit(&pos, &bricks, (210.0, 10.0), None), None);
    }
}
