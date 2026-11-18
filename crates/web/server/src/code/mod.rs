//! brickie's adapter for the code viewer: the `codeview` provider port, implemented from the
//! config, the cache and the check result.
//!
//! The generic viewer (`codeview-host`) knows nothing of bricks; this brick answers its
//! questions. Every call loads the config, cache and diagnostics again, like every other route,
//! and returns an empty answer (an empty viewer) when they cannot be loaded. It is not
//! the viewer and does no HTTP.
//!
//! # Usage
//!
//! ```ignore
//! // Crate-private: mounted by `routes::router` at `/code`.
//! let provider = std::sync::Arc::new(CodeProvider::new("brickie.toml".into()));
//! let app = codeview_host::router(provider);
//! ```
//!
//! The viewer then asks the provider for, per request:
//!
//! - `views`: the explorer trees (Bricks and Folders) with finding badges (`views`);
//! - file text (`read`), confined to the workspace root;
//! - findings as marks on lines (`marks`);
//! - function complexity trees as gutter bands (`bands`);
//! - definition and references from the cached reference edges (`nav`).
//!
//! # Invariants
//!
//! A path outside the workspace is never read. Marks use the same diagnostics as
//! `brickie check`, so the viewer and the CLI never disagree.

mod bands;
mod marks;
mod nav;
mod read;
mod views;

use brickie_check::check::Loaded;
use codeview_port::{Band, FileText, Location, Mark, Pos, Provider, View};
use std::path::PathBuf;

/// The provider for the workspace of the config at `config`.
pub struct CodeProvider {
    config: PathBuf,
}

impl CodeProvider {
    pub fn new(config: PathBuf) -> Self {
        CodeProvider { config }
    }

    /// Config, cache and diagnostics; `None` (an empty viewer) when they cannot be loaded.
    fn load(&self) -> Option<Loaded> {
        brickie_change::working::load(&self.config)
            .ok()
            .map(|(l, _)| l)
    }
}

impl Provider for CodeProvider {
    fn views(&self) -> Vec<View> {
        self.load().map(|l| views::views(&l)).unwrap_or_default()
    }

    fn file(&self, path: &str) -> Option<FileText> {
        read::file(&self.load()?, path)
    }

    fn marks(&self, path: &str) -> Vec<Mark> {
        self.load()
            .map(|l| marks::marks(&l, path))
            .unwrap_or_default()
    }

    fn bands(&self, path: &str) -> Vec<Band> {
        self.load()
            .map(|l| bands::bands(&l, path))
            .unwrap_or_default()
    }

    fn definition(&self, path: &str, pos: Pos) -> Vec<Location> {
        self.load()
            .map(|l| nav::definition(&l, path, pos))
            .unwrap_or_default()
    }

    fn references(&self, path: &str, pos: Pos) -> Vec<Location> {
        self.load()
            .map(|l| nav::references(&l, path, pos))
            .unwrap_or_default()
    }
}
