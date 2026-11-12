//! Matching a module against a brick member, with the specificity that picks the winner.

use crate::cache::{Crate, Module};
use std::path::{Component, Path, PathBuf};

/// How specifically a member matches: the kind (`path:` 0 < package 1 < module 2), then its
/// length (path components or `::` segments). Higher wins.
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
}
