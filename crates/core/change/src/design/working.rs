//! Which design `check` reads: the draft when one exists, else the published files.

use super::Design;
use anyhow::Result;
use brickie_check::check::Loaded;
use brickie_model::design::Track;
use serde::Serialize;
use std::path::Path;

/// The source of a working load: the track and versions, for the header line of `check`.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Working {
    /// Which tree was read.
    pub track: Track,
    /// The head of that track (0 when no version is recorded yet, as before a first design edit).
    pub v: u64,
    /// The published version a draft is based on.
    pub forked_from: Option<u64>,
}

impl Working {
    /// `draft v42 from published v37` or `published v37`.
    pub fn name(&self) -> String {
        let track = self.track.name();
        let v = match self.v {
            0 => String::new(),
            v => format!(" v{v}"),
        };
        match self.forked_from {
            Some(f) => format!("{track}{v} from published v{f}"),
            None => format!("{track}{v}"),
        }
    }
}

/// Load the working design of `config` against the measured code: the draft when one exists,
/// else the published files. There is no other source.
pub fn load(config: &Path) -> Result<(Loaded, Working)> {
    Design::at(config)?.load_checked()
}
