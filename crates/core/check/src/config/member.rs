//! Matching a module against a brick member.

use brickie_facts::{Crate, Module};
use std::path::{Component, Path, PathBuf};

/// Facts about one module that members are matched against.
pub struct Subject<'a> {
    pub module: &'a Module,
    /// Absolute source file of the module.
    pub file: PathBuf,
    pub package: &'a str,
    /// Absolute package directory (workspace root joined with the crate's `dir`).
    pub dir: PathBuf,
}

impl<'a> Subject<'a> {
    pub fn new(module: &'a Module, krate: Option<&'a Crate>, ws_root: &Path) -> Self {
        Subject {
            module,
            file: normalise(&ws_root.join(&module.file)),
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

/// Whether `member` (declared in a file under `base`) matches `s`: a package name (every module
/// of it), a module prefix, or a `path:` (every module of a crate under it, or whose source file
/// is under it, so `path:src/lib.rs` names one crate root).
pub fn matches(member: &str, base: &Path, s: &Subject) -> bool {
    if let Some(dir) = member.strip_prefix("path:") {
        let dir = normalise(&base.join(dir));
        return s.dir.starts_with(&dir) || s.file.starts_with(&dir);
    }
    if member == s.package {
        return true;
    }
    let id = &s.module.id;
    id == member || id.strip_prefix(member).is_some_and(|r| r.starts_with("::"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_match_paths_packages_and_module_prefixes() {
        let m = Module {
            id: "web-api::routes::x".into(),
            krate: "web-api".into(),
            file: "crates/web/api/src/routes/x.rs".into(),
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
        let s = Subject::new(&m, Some(&c), Path::new("/ws"));
        let base = Path::new("/ws");
        for yes in [
            "path:crates/web/",
            "path:crates/web/api",
            "web-api",
            "web-api::routes",
        ] {
            assert!(matches(yes, base, &s), "{yes}");
        }
        assert!(!matches("path:crates/webx/", base, &s));
        assert!(matches("path:../crates/web", Path::new("/ws/sub"), &s));
        assert!(!matches("web-api::route", base, &s));
        // A file or folder inside the crate matches by the module's source file.
        assert!(matches("path:crates/web/api/src/routes/", base, &s));
        assert!(matches("path:crates/web/api/src/routes/x.rs", base, &s));
        assert!(!matches("path:crates/web/api/src/lib.rs", base, &s));
    }
}
