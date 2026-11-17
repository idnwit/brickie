//! The `[[link]]`s of each level (`links` of a `LevelView`), judged by the shared decisions in
//! `brickie_check`: denied and cycle flags always; view mode also marks the planned ones (valid
//! links that no measured edge uses yet). Design mode reads the design files alone.

use brickie_check::check::{Loaded, design_links, marked_links};
use brickie_check::config::{LinkState, level_name};
use brickie_web_api::LevelLink;
use std::path::{Path, PathBuf};

/// Every link of the design with its planned flag, in tree order.
pub fn served(l: &Loaded) -> Vec<(String, LevelLink)> {
    as_served(l, &marked_links(l))
}

/// Every link of the design files, none planned (Design mode reads no code).
pub fn design(l: &Loaded) -> Vec<(String, LevelLink)> {
    as_served(l, &design_links(&l.cfg))
}

fn as_served(l: &Loaded, links: &[LinkState]) -> Vec<(String, LevelLink)> {
    let root = root_dir(&l.cfg.file);
    links
        .iter()
        .map(|s| (level_name(&s.level), edge(s, &root)))
        .collect()
}

fn edge(s: &LinkState, root: &Path) -> LevelLink {
    let k = &s.link;
    let loc = if k.loc.is_empty() { &s.file } else { &k.loc };
    LevelLink {
        level: level_name(&s.level),
        file: rel(root, loc),
        from: k.from.clone(),
        to: k.to.clone(),
        kinds: k.kinds.clone(),
        scope: k.scope.name().to_string(),
        reason: k.reason.clone(),
        filter: s.match_text(),
        denied: s.denied.clone(),
        cycle: s.cycle,
        planned: s.planned,
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
