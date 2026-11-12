//! The measure cache (`.brickie/cache.json`): facts only, no rules.

mod files;
mod hash;
mod model;

pub use files::{crate_dirs, walk_rs};
pub use hash::sha256_hex;
pub use model::*;
