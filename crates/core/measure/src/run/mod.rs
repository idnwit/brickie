//! `brickie measure`: cargo metadata + rust-analyzer SCIP (dependencies) + tree-sitter
//! (complexity) -> `.brickie/cache.json`.

mod complexity;
mod edges;
mod files;
mod items;
mod reach;

pub use complexity::fn_tree;

use crate::scip::{self, Graph, layout};
use crate::workspace::lock::LockGuard;
use crate::workspace::{crates, git, metadata, rust_analyzer};
use anyhow::{Context, Result};
use brickie_facts::{Cache, CacheLock, SCHEMA_VERSION};
use protobuf::Message;
use std::path::{Path, PathBuf};

pub struct Options {
    pub config: PathBuf,
    pub scip: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
}

/// Measure and write the cache; returns its path.
pub fn run(opts: &Options) -> Result<PathBuf> {
    run_with(opts, &|_| {})
}

/// [`run`], reporting each step's name to `progress` as it starts. Holds the cache lock
/// ([`CacheLock`]) for the whole run: a concurrent measure of the same config fails at once.
pub fn run_with(opts: &Options, progress: &dyn Fn(&str)) -> Result<PathBuf> {
    let cfg_dir = opts
        .config
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    // The config only locates the cache; bricks are resolved by `check` and `serve`.
    if !opts.config.exists() && opts.config != Path::new("brickie.toml") {
        anyhow::bail!("config {} not found", opts.config.display());
    }
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
    let scip_path = match &opts.scip {
        Some(p) => p.clone(),
        None => {
            let out = tmp.path().join("index.scip");
            progress("rust-analyzer scip");
            rust_analyzer::run(&root, &out)?;
            out
        }
    };
    progress("read index");
    let bytes =
        std::fs::read(&scip_path).with_context(|| format!("read SCIP {}", scip_path.display()))?;
    let idx = ::scip::types::Index::parse_from_bytes(&bytes).context("decode SCIP")?;
    drop(bytes);
    let g = scip::normalise(&idx, &cargo_layout(&ws), &|p| {
        std::fs::read_to_string(root.join(p)).ok()
    });

    progress("analyse files");
    let cache = build(&g, &ws, idx.metadata.tool_info.version.clone(), &root);
    progress("write cache");
    // Write then rename, so readers never see a half-written cache.
    let out = dir.join("cache.json");
    let part = dir.join("cache.json.part");
    std::fs::write(&part, serde_json::to_vec_pretty(&cache)?)?;
    std::fs::rename(&part, &out).with_context(|| format!("write {}", out.display()))?;
    Ok(out)
}

fn build(g: &Graph, ws: &metadata::Workspace, ra_version: String, root: &Path) -> Cache {
    let crate_ids = crates::crate_ids(ws);
    let crates = crates::crates(ws, &crate_ids);
    let (module_ids, module_list) = scip::modules::modules(g, &crate_ids);
    let (sources, analyses) = files::analyze_all(g, root);
    let test_set = files::test_files(&analyses);
    let mut files = files::compiled_files(g, &module_ids, &sources, &analyses, &test_set);
    files::add_uncompiled(&mut files, &crates, root);
    edges::add_edges(&mut files, g, &module_ids, &analyses, &test_set);
    items::add_items(&mut files, g, &module_ids, &crate_ids, &analyses, &test_set);

    Cache {
        schema_version: SCHEMA_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        rust_analyzer: ra_version,
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
            })
            .collect(),
    }
}
