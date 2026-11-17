//! The rule links of a level (`unused_links` of a `LevelView`): in view mode the allow rules
//! between named bricks that no measured edge backs; in Design mode every rule between named
//! bricks, allow or deny, read from the design files alone.

use brickie_check::check::{Loaded, rule_links, unused_links};
use brickie_check::config::level_name;
use brickie_check::config::{Action, Link};
use brickie_web_api::UnusedLink;
use std::path::{Path, PathBuf};

/// Every unused link of the design as a served edge, in tree order.
pub fn served(l: &Loaded) -> Vec<(String, UnusedLink)> {
    as_served(l, &unused_links(l), Link::filter_text)
}

/// Every rule between named bricks of the design as a served edge, in tree order.
pub fn rules(l: &Loaded) -> Vec<(String, UnusedLink)> {
    as_served(l, &rule_links(&l.cfg), Link::match_text)
}

/// `links` as served edges; `filter` writes each one's filter text.
fn as_served(l: &Loaded, links: &[Link], filter: fn(&Link) -> String) -> Vec<(String, UnusedLink)> {
    let root = root_dir(&l.cfg.file);
    links
        .iter()
        .map(|k| (level_name(&k.level), edge(k, &root, filter)))
        .collect()
}

fn edge(k: &Link, root: &Path, filter: fn(&Link) -> String) -> UnusedLink {
    UnusedLink {
        level: level_name(&k.level),
        file: rel(root, &k.loc),
        from: k.from.clone(),
        to: k.to.clone(),
        kinds: k.kinds.clone(),
        scope: k.scope.name().to_string(),
        action: match k.action {
            Action::Allow => "allow",
            Action::Deny => "deny",
        }
        .into(),
        reason: k.reason.clone().filter(|s| !s.trim().is_empty()),
        filter: filter(k),
    }
}

/// The directory of the root config file.
fn root_dir(file: &str) -> PathBuf {
    let abs = std::path::absolute(file).unwrap_or_else(|_| PathBuf::from(file));
    abs.parent().map(Path::to_path_buf).unwrap_or_default()
}

/// A `file:line` location as a `/`-separated path relative to `root` (the path as given when
/// it is outside).
fn rel(root: &Path, loc: &str) -> String {
    let file = match loc.rsplit_once(':') {
        Some((f, n)) if n.parse::<u32>().is_ok() => f,
        _ => loc,
    };
    let abs = std::path::absolute(file).unwrap_or_else(|_| PathBuf::from(file));
    match abs.strip_prefix(root) {
        Ok(p) => p
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => file.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_become_root_relative_paths() {
        let root = Path::new("/w/proj");
        assert_eq!(rel(root, "/w/proj/web/brickie.toml:12"), "web/brickie.toml");
        assert_eq!(rel(root, "/other/x.toml:3"), "/other/x.toml");
    }
}
