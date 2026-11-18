//! The measure flow: index, analyse, assign and write `.brickie/cache.json`.
//!
//! The flow takes the cargo workspace facts and one rust-analyzer SCIP index per target (the
//! host first, then each additional target), normalises each index into rows, reads every
//! source file for hash and function complexity, assigns edges and public items to modules and
//! merges the per-target caches. It records facts only; rules and bricks are applied later by
//! `brickie-check`.
//!
//! # Usage
//!
//! The crate exports [`run_with`], [`Options`] and [`fn_tree`]. Because the module is a folder
//! brick of the crate, the example is a flow sketch rather than a doctest:
//!
//! ```text
//! let opts = Options { config: "brickie.toml".into(), scip: None, workspace: None,
//!                      additional_targets: vec![] };
//! let cache_path = run_with(&opts, &|step| eprintln!("{step}"))?;   // .brickie/cache.json
//! ```
//!
//! Steps reported to the callback, in order: `cargo metadata`, then per target
//! `rust-analyzer scip`, `read index`, `analyse files`, and finally `write cache`.
//!
//! # Key items
//!
//! - [`Options`]: which config, optional pre-made index, workspace and extra targets.
//! - [`run_with`]: the whole flow; returns the cache path.
//! - [`fn_tree`]: the decision tree of one function in a source text.
//!
//! # Notes
//!
//! - A given `scip` index stands for the host only: extra targets are not measured and none is
//!   recorded in the cache.
//! - Files that cannot be read stay in the cache as empty rows and are reported as warnings.
//! - The cache lock and the `Cargo.lock` restore guard are held for the whole run.

mod complexity;
#[cfg(test)]
mod e2e_tests;
mod edges;
mod files;
mod hint;
mod items;
mod merge;
mod reach;

pub use complexity::fn_tree;

use crate::scip::{self, Graph, layout};
use crate::workspace::lock::LockGuard;
use crate::workspace::{crates, git, metadata, rust_analyzer};
use anyhow::{Context, Result};
use brickie_facts::{Cache, CacheLock, Measured, SCHEMA_VERSION};
use protobuf::Message;
use std::path::{Path, PathBuf};

/// What to measure and where to put it.
pub struct Options {
    /// The `brickie.toml` path; only its directory is used, to place `.brickie/` (bricks are
    /// resolved later by `check`).
    pub config: PathBuf,
    /// A ready SCIP index to use instead of running rust-analyzer; it stands for the host only.
    pub scip: Option<PathBuf>,
    /// The workspace to measure; defaults to the config's directory.
    pub workspace: Option<PathBuf>,
    /// Targets measured besides the host (`[measure] additional_targets`).
    pub additional_targets: Vec<String>,
}

/// Measure and write the cache, reporting each step's name to `progress` as it starts. Returns
/// the cache path (`<config dir>/.brickie/cache.json`).
///
/// Holds the cache lock ([`CacheLock`]) for the whole run: a concurrent measure of the same
/// config fails at once. Errors when the config path is missing, cargo or rust-analyzer fails,
/// or the index cannot be decoded.
pub fn run_with(opts: &Options, progress: &dyn Fn(&str)) -> Result<PathBuf> {
    let cfg_dir = config_dir(opts)?;
    let dir = cfg_dir.join(".brickie");
    let _cache_lock = CacheLock::acquire(&dir).map_err(anyhow::Error::msg)?;
    let workspace = opts.workspace.clone().unwrap_or_else(|| cfg_dir.clone());
    let workspace = workspace
        .canonicalize()
        .with_context(|| format!("workspace {}", workspace.display()))?;

    // Never leave a changed Cargo.lock behind in the measured workspace.
    let _lock = LockGuard::new(&workspace);
    progress("cargo metadata");
    let ws = metadata::load(&workspace)?;
    let root = ws.root.clone();
    let _root_lock = (root != workspace).then(|| LockGuard::new(&root));

    let tmp = tempfile::tempdir().context("temp dir")?;
    // One index per target: the host (`None`), then each additional target.
    let runs: Vec<Option<&str>> = std::iter::once(None)
        .chain(opts.additional_targets.iter().map(|t| Some(t.as_str())))
        .collect();
    let mut unreadable = Vec::new();
    let mut merged: Option<Cache> = None;
    for (i, target) in runs.iter().enumerate() {
        let scip_path = match &opts.scip {
            Some(p) => p.clone(),
            None => {
                let out = tmp.path().join(format!("index{i}.scip"));
                progress(&match target {
                    Some(t) => format!("rust-analyzer scip ({t})"),
                    None => "rust-analyzer scip".to_string(),
                });
                rust_analyzer::run(&root, &out, *target)?;
                out
            }
        };
        progress("read index");
        let bytes = std::fs::read(&scip_path)
            .with_context(|| format!("read SCIP {}", scip_path.display()))?;
        let idx = ::scip::types::Index::parse_from_bytes(&bytes).context("decode SCIP")?;
        drop(bytes);
        let (g, mut unread) = scip::normalise(&idx, &cargo_layout(&ws), &|p| {
            std::fs::read_to_string(root.join(p)).ok()
        });
        progress("analyse files");
        let cache = build(
            &g,
            &ws,
            idx.metadata.tool_info.version.clone(),
            &root,
            &mut unread,
        );
        unreadable.extend(unread);
        match &mut merged {
            Some(base) => merge::merge(base, cache),
            None => merged = Some(cache),
        }
        if opts.scip.is_some() {
            break; // a given index stands for the host only
        }
    }
    let mut cache = merged.context("no index was measured")?;
    cache.measured = measured_by(opts);
    cache.measured.location = (cfg_dir.canonicalize().unwrap_or(cfg_dir))
        .display()
        .to_string();
    // Unreadable files stay in the cache as empty rows; say so rather than hide it.
    unreadable.sort();
    unreadable.dedup();
    for path in &unreadable {
        progress(&format!("warning: cannot read {path}; recorded as empty"));
    }
    for line in target_hints(&root, &cache) {
        progress(&line);
    }
    progress("write cache");
    write_cache(&dir, &cache)
}

/// The directory of the config, which holds the `.brickie` cache folder.
fn config_dir(opts: &Options) -> Result<PathBuf> {
    // The config only locates the cache; bricks are resolved by `check` and `serve`.
    if !opts.config.exists() && opts.config != Path::new("brickie.toml") {
        anyhow::bail!("config {} not found", opts.config.display());
    }
    Ok(opts
        .config
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf))
}

/// Write then rename, so readers never see a half-written cache.
fn write_cache(dir: &Path, cache: &Cache) -> Result<PathBuf> {
    let out = dir.join("cache.json");
    let part = dir.join("cache.json.part");
    std::fs::write(&part, serde_json::to_vec_pretty(cache)?)?;
    std::fs::rename(&part, &out).with_context(|| format!("write {}", out.display()))?;
    Ok(out)
}

/// What the run measured: a given index stands for the host only, so it lists no targets.
fn measured_by(opts: &Options) -> Measured {
    if opts.scip.is_some() {
        Measured::all(Vec::new())
    } else {
        Measured::all(opts.additional_targets.clone())
    }
}

/// The cache; paths that could not be read are appended to `unreadable`.
fn build(
    g: &Graph,
    ws: &metadata::Workspace,
    ra_version: String,
    root: &Path,
    unreadable: &mut Vec<String>,
) -> Cache {
    let crate_ids = crates::crate_ids(ws);
    let crates = crates::crates(ws, &crate_ids);
    let (module_ids, module_list) = scip::modules::modules(g, &crate_ids);
    let (sources, analyses, unread) = files::analyze_all(g, root);
    unreadable.extend(unread);
    let test_set = files::test_files(&analyses);
    let mut files = files::compiled_files(g, &module_ids, &sources, &analyses, &test_set);
    unreadable.extend(files::add_uncompiled(&mut files, &crates, root));
    edges::add_edges(&mut files, g, &module_ids, &analyses, &test_set);
    items::add_items(&mut files, g, &module_ids, &crate_ids, &analyses, &test_set);

    Cache {
        schema_version: SCHEMA_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        rust_analyzer: ra_version,
        measured: Measured::default(),
        workspace_root: root.display().to_string(),
        git: git::git_info(root),
        crates,
        modules: module_list,
        files,
    }
}

/// The package directories and target roots the SCIP rows are assigned to.
fn cargo_layout(ws: &metadata::Workspace) -> layout::Layout {
    layout::Layout {
        packages: ws
            .packages
            .iter()
            .map(|p| layout::Package { dir: p.dir.clone() })
            .collect(),
        targets: ws
            .targets
            .iter()
            .map(|t| layout::Target {
                package: t.package,
                name: t.name.clone(),
                src_path: t.src_path.clone(),
                lib: crate::workspace::crates::is_lib_kind(&t.kind),
            })
            .collect(),
    }
}

/// The host triple from `rustc -vV` (run in the workspace so rustup picks its toolchain).
fn host_triple(root: &Path) -> Option<String> {
    let out = std::process::Command::new("rustc")
        .arg("-vV")
        .current_dir(root)
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("host: "))
        .map(|h| h.trim().to_string())
}

/// Hint lines for `cfg(target_*)` code that neither the host nor an additional target covers.
fn target_hints(root: &Path, cache: &Cache) -> Vec<String> {
    let mut cfgs = std::collections::BTreeSet::new();
    for path in cache.files.keys() {
        if let Ok(text) = std::fs::read_to_string(root.join(path)) {
            cfgs.extend(hint::target_cfgs(&text));
        }
    }
    // Without the host triple every family would look uncovered, so give no hints.
    let Some(host) = host_triple(root) else {
        return Vec::new();
    };
    let mut triples = cache.measured.targets.clone();
    triples.push(host);
    hint::hints(&cfgs, &triples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_index_records_no_targets() {
        let mut o = Options {
            config: PathBuf::from("brickie.toml"),
            scip: None,
            workspace: None,
            additional_targets: vec!["wasm32-unknown-unknown".into()],
        };
        assert_eq!(measured_by(&o).targets, ["wasm32-unknown-unknown"]);
        o.scip = Some(PathBuf::from("x.scip"));
        assert!(measured_by(&o).targets.is_empty());
    }
}
