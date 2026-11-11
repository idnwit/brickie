//! `brickie measure`: cargo metadata + rust-analyzer SCIP (dependencies) + tree-sitter
//! (complexity) -> `.brickie/cache.json`.

mod complexity;
mod crates;
mod edges;
mod files;
mod git;
mod lock;
mod metadata;
mod modules;
mod rust_analyzer;
mod scip;

pub use complexity::fn_tree;

use crate::cache::{Bricks, Cache, SCHEMA_VERSION};
use crate::config;
use anyhow::{Context, Result};
use lock::LockGuard;
use protobuf::Message;
use scip::Graph;
use std::path::{Path, PathBuf};

pub struct Options {
    pub config: PathBuf,
    pub scip: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
}

pub fn run(opts: &Options) -> Result<PathBuf> {
    let cfg_dir = opts
        .config
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let bricks = if opts.config.exists() {
        config::load(&opts.config)?.bricks
    } else if opts.config == Path::new("brickie.toml") {
        eprintln!("brickie: no brickie.toml, measuring without bricks");
        Vec::new()
    } else {
        anyhow::bail!("config {} not found", opts.config.display());
    };
    let workspace = opts.workspace.clone().unwrap_or_else(|| cfg_dir.clone());
    let workspace = workspace
        .canonicalize()
        .with_context(|| format!("workspace {}", workspace.display()))?;

    // Never leave a changed Cargo.lock behind in the measured workspace.
    let _lock = LockGuard::new(&workspace);
    let ws = metadata::load(&workspace)?;
    let root = ws.root.clone();
    let _root_lock = (root != workspace).then(|| LockGuard::new(&root));

    let tmp = tempfile::tempdir().context("temp dir")?;
    let scip_path = match &opts.scip {
        Some(p) => p.clone(),
        None => {
            let out = tmp.path().join("index.scip");
            rust_analyzer::run(&root, &out)?;
            out
        }
    };
    let bytes =
        std::fs::read(&scip_path).with_context(|| format!("read SCIP {}", scip_path.display()))?;
    let idx = ::scip::types::Index::parse_from_bytes(&bytes).context("decode SCIP")?;
    drop(bytes);
    let g = scip::normalise(&idx, &ws, &|p| std::fs::read_to_string(root.join(p)).ok());

    let cache = build(
        &g,
        &ws,
        bricks,
        idx.metadata.tool_info.version.clone(),
        &root,
    )?;
    let dir = cfg_dir.join(".brickie");
    std::fs::create_dir_all(&dir)?;
    let out = dir.join("cache.json");
    std::fs::write(&out, serde_json::to_vec_pretty(&cache)?)?;
    Ok(out)
}

fn build(
    g: &Graph,
    ws: &metadata::Workspace,
    bricks: Vec<config::Brick>,
    ra_version: String,
    root: &Path,
) -> Result<Cache> {
    let crate_ids = crates::crate_ids(ws);
    let crates = crates::crates(ws, &crate_ids);
    let (module_ids, module_list) = modules::modules(g, &crate_ids);
    let (sources, analyses) = files::analyze_all(g, root);
    let test_set = files::test_files(&analyses);
    let mut files = files::compiled_files(g, &module_ids, &sources, &analyses, &test_set);
    files::add_uncompiled(&mut files, &crates, root);
    edges::add_edges(&mut files, g, &module_ids, &analyses, &test_set);

    let (module_brick, unmatched) = config::resolve(&bricks, &crates, &module_list)?;
    for u in unmatched {
        eprintln!("brickie: warning: brick member matches no module ({u})");
    }
    Ok(Cache {
        schema_version: SCHEMA_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        rust_analyzer: ra_version,
        workspace_root: root.display().to_string(),
        git: git::git_info(root),
        bricks: Bricks {
            config_hash: config::hash(&bricks),
            defs: bricks,
            module_brick,
        },
        crates,
        modules: module_list,
        files,
    })
}
