//! The side panels: findings list and brick index (left), details, functions and legend (right).

mod bricks;
mod card;
mod details;
mod diagnostics;
mod functions;
mod legend;
mod node;

pub use details::detail;
pub use diagnostics::{current_list, left};

/// Finding groups: layer key and the name shown.
pub const GROUPS: [(&str, &str); 4] = [
    ("deps", "deps"),
    ("cx", "complexity"),
    ("stale", "stale"),
    ("iface", "interface"),
];
