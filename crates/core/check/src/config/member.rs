//! Matching a module against a brick member, with the specificity that picks the winner.

use brickie_facts::{Crate, Module};
use std::path::{Component, Path, PathBuf};

/// How specifically a member matches: the kind (`path:` 0 < package 1 < module prefix 2 < exact
/// module 3), then its length (path components or `::` segments). Higher wins.
pub type Spec = (u8, usize);

/// Facts about one module that members are matched against.
pub struct Subject<'a> {
    pub module: &'a Module,
    pub package: &'a str,
    /// Absolute package directory (workspace root joined with the crate's `dir`).
    pub dir: PathBuf,
}

impl<'a> Subject<'a> {
    pub fn new(module: &'a Module, krate: Option<&'a Crate>, ws_root: &Path) -> Self {
        Subject {
            module,
            package: krate.map_or("", |c| c.package.as_str()),
            dir: normalise(&ws_root.join(krate.map_or("", |c| c.dir.as_str()))),
        }
    }
}

/// Lexical normalisation: drops `.` and resolves `..` without touching the file system.
pub fn normalise(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// The specificity of `member` (declared in a file under `base`) for `s`, or `None`.
pub fn matches(member: &str, base: &Path, s: &Subject) -> Option<Spec> {
    if let Some(dir) = member.strip_prefix("path:") {
        let dir = normalise(&base.join(dir));
        return s
            .dir
            .starts_with(&dir)
            .then(|| (0, dir.components().count()));
    }
    if member == s.package {
        return Some((1, 0));
    }
    if let Some(id) = member.strip_prefix("module:") {
        return (s.module.id == id).then(|| (3, id.matches("::").count()));
    }
    let id = &s.module.id;
    let under = id == member || id.strip_prefix(member).is_some_and(|r| r.starts_with("::"));
    under.then(|| (2, member.matches("::").count()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specificity_orders_path_package_module() {
        let m = Module {
            id: "web-api::routes::x".into(),
            krate: "web-api".into(),
            file: String::new(),
            line: 1,
        };
        let c = Crate {
            id: "web-api".into(),
            package: "web-api".into(),
            version: "0".into(),
            target: "web-api".into(),
            kind: "lib".into(),
            root: "crates/web/api/src/lib.rs".into(),
            dir: "crates/web/api".into(),
        };
        let ws = Path::new("/ws");
        let s = Subject::new(&m, Some(&c), ws);
        let base = Path::new("/ws");
        let short = matches("path:crates/web/", base, &s).unwrap();
        let long = matches("path:crates/web/api", base, &s).unwrap();
        let pkg = matches("web-api", base, &s).unwrap();
        let module = matches("web-api::routes", base, &s).unwrap();
        assert!(short < long && long < pkg && pkg < module);
        assert!(matches("path:crates/webx/", base, &s).is_none());
        assert!(matches("path:../crates/web", Path::new("/ws/sub"), &s).is_some());
        assert!(matches("web-api::route", base, &s).is_none());
    }

    #[test]
    fn exact_module_matches_only_that_module() {
        let root = Module {
            id: "app".into(),
            krate: "app".into(),
            file: String::new(),
            line: 1,
        };
        let child = Module {
            id: "app::parent".into(),
            krate: "app".into(),
            file: String::new(),
            line: 1,
        };
        let other_target = Module {
            id: "app/bin/tool".into(),
            krate: "app".into(),
            file: String::new(),
            line: 1,
        };
        let c = Crate {
            id: "app".into(),
            package: "app".into(),
            version: "0".into(),
            target: "app".into(),
            kind: "lib".into(),
            root: "crates/app/src/lib.rs".into(),
            dir: "crates/app".into(),
        };
        let ws = Path::new("/ws");
        let s = Subject::new(&root, Some(&c), ws);
        assert_eq!(matches("module:app", ws, &s), Some((3, 0)));
        assert!(matches("module:app::parent", ws, &s).is_none());
        assert!(matches("app::parent", ws, &s).is_none());

        let child_s = Subject::new(&child, Some(&c), ws);
        assert_eq!(matches("module:app::parent", ws, &child_s), Some((3, 1)));
        assert!(matches("module:app", ws, &child_s).is_none());
        assert!(matches("module:app::parent", ws, &s).is_none());

        let target_s = Subject::new(&other_target, Some(&c), ws);
        assert!(matches("module:app", ws, &target_s).is_none());
    }

    #[test]
    fn exact_module_beats_a_module_prefix() {
        let m = Module {
            id: "app::parent".into(),
            krate: "app".into(),
            file: String::new(),
            line: 1,
        };
        let c = Crate {
            id: "app".into(),
            package: "app".into(),
            version: "0".into(),
            target: "app".into(),
            kind: "lib".into(),
            root: "src/lib.rs".into(),
            dir: "".into(),
        };
        let s = Subject::new(&m, Some(&c), Path::new("/ws"));
        assert!(
            matches("module:app::parent", Path::new("/ws"), &s).unwrap()
                > matches("app::parent", Path::new("/ws"), &s).unwrap()
        );
    }
}
