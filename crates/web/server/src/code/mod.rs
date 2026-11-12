//! brickie's adapter for the code viewer (docs/proposals/code-viewer.md): implements the
//! `codeview` provider port from the config, the cache and the check result, read per request
//! like every other route. Trees in `views`, file text in `read`, findings as marks in `marks`,
//! function complexity trees as gutter bands in `bands`,
//! definition and references from the cached reference edges in `nav`.

mod bands;
mod marks;
mod nav;
mod read;
mod views;

use brickie_check::check::{self, Loaded};
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
        check::load(&self.config).ok()
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
