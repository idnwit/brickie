//! Ops the editor's gestures and fields build, and the paths and keys they name bricks by.

use brickie_web_api::BrickView;
use brickie_web_api::change::{BrickKind, BrickSpec, Op};

/// The brick path from the root (without `root`) of brick `name` at level `lv`.
pub fn brick_path(lv: &str, name: &str) -> Vec<String> {
    lv.split('/')
        .skip(1)
        .map(str::to_string)
        .chain(std::iter::once(name.to_string()))
        .collect()
}

/// The brick path (without `root`) of level `lv` itself (`[]`: the root).
pub fn level_brick_path(lv: &str) -> Vec<String> {
    lv.split('/').skip(1).map(str::to_string).collect()
}

/// The layout key (level path) of brick `name` at level `lv`.
pub fn layout_key(lv: &str, name: &str) -> String {
    format!("{lv}/{name}")
}

/// The toolbar's `add_brick` (a module names its crate in `members`, `crate::module`).
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

/// `a, b ,c` -> `[a, b, c]`.
pub fn split_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// A brick field the inspector edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Desc(String),
    Tags(Vec<String>),
    Members(Vec<String>),
    Path(String),
}

/// `set_brick` with the one changed field; `None` when it equals what `b` has.
pub fn set_field(file: &str, b: &BrickView, f: Field) -> Option<Op> {
    let mut op = Op::SetBrick {
        file: file.to_string(),
        name: b.name.clone(),
        desc: None,
        tags: None,
        members: None,
        path: None,
    };
    let Op::SetBrick {
        desc,
        tags,
        members,
        path,
        ..
    } = &mut op
    else {
        return None;
    };
    match f {
        Field::Desc(d) if d.trim() != b.desc.as_deref().unwrap_or("") => {
            *desc = Some(d.trim().to_string())
        }
        Field::Tags(t) if t != b.tags => *tags = Some(t),
        Field::Members(m) if m != b.members => *members = Some(m),
        // The view does not carry the written `path`: any non-empty value is sent.
        Field::Path(p) if !p.trim().is_empty() => *path = Some(p.trim().to_string()),
        _ => return None,
    }
    Some(op)
}

/// `rename_brick`, `None` when the name is empty or unchanged.
pub fn rename_op(file: &str, from: &str, to: &str) -> Option<Op> {
    let to = to.trim();
    (!to.is_empty() && to != from).then(|| Op::RenameBrick {
        file: file.to_string(),
        from: from.to_string(),
        to: to.to_string(),
    })
}

/// `vec` with `item` added (once) or removed.
pub fn toggled(vec: &[String], item: &str, on: bool) -> Vec<String> {
    let mut out: Vec<String> = vec.iter().filter(|x| *x != item).cloned().collect();
    if on && !item.trim().is_empty() {
        out.push(item.trim().to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_keys() {
        assert_eq!(brick_path("root", "cli"), vec!["cli"]);
        assert_eq!(brick_path("root/web", "api"), vec!["web", "api"]);
        assert!(level_brick_path("root").is_empty());
        assert_eq!(layout_key("root/web", "api"), "root/web/api");
        assert_eq!(split_list(" a, ,b "), vec!["a", "b"]);
    }

    #[test]
    fn fields_apply_only_when_changed() {
        let b = BrickView {
            name: "api".into(),
            tags: vec!["io".into()],
            ..Default::default()
        };
        assert!(set_field("x.toml", &b, Field::Tags(vec!["io".into()])).is_none());
        let op = set_field("x.toml", &b, Field::Desc(" hi ".into())).unwrap();
        let v = serde_json::to_value(&op).unwrap();
        assert_eq!((v["desc"].as_str(), v.get("tags")), (Some("hi"), None));
        assert!(rename_op("x", "api", " api ").is_none());
        assert!(rename_op("x", "api", "web").is_some());
        assert_eq!(toggled(&b.tags, "db", true), vec!["io", "db"]);
        assert!(toggled(&b.tags, "io", false).is_empty());
        let a = add_brick_op("brickie.toml", BrickKind::Crate, " cache ", "");
        assert_eq!(serde_json::to_value(&a).unwrap()["brick"]["name"], "cache");
    }
}
