//! The planned and unused links of a level: the allow rules between named bricks that no
//! measured edge backs, with the state `brickie-check` computed (`link_states`). The viewer
//! draws them in view mode and in the editor alike.

use brickie_check::check::{Link, LinkState, Loaded, filter_text, link_states};
use brickie_check::config::level_name;
use brickie_web_api::PlannedEdge;
use std::path::{Path, PathBuf};

/// Every link of the design as a served edge, in tree order.
pub fn planned_edges(l: &Loaded) -> Vec<(String, PlannedEdge)> {
    let root = root_dir(&l.cfg.file);
    link_states(l)
        .iter()
        .map(|k| (level_name(&k.level), edge(k, &root)))
        .collect()
}

fn edge(k: &Link, root: &Path) -> PlannedEdge {
    PlannedEdge {
        level: level_name(&k.level),
        file: rel(root, &k.loc),
        from: k.from.clone(),
        to: k.to.clone(),
        kinds: k.kinds.clone(),
        scope: k.scope.name().to_string(),
        action: "allow".into(),
        reason: k.reason.clone().filter(|s| !s.trim().is_empty()),
        state: state_name(k.state).into(),
        filter: filter_text(k),
    }
}

/// The served name of a link state (`planned`, `unused`).
pub fn state_name(s: LinkState) -> &'static str {
    s.name()
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

    #[test]
    fn states_are_served_by_name() {
        assert_eq!(state_name(LinkState::Planned), "planned");
        assert_eq!(state_name(LinkState::Unused), "unused");
    }
}
