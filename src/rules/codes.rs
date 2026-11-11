//! Every diagnostic code brickie can emit. `docs/llms.txt` must document each of them (a test
//! in `cli/llms.rs` enforces it), so a new code is added here first.

pub const DEPS_DENY: &str = "deps/deny";
pub const DEPS_DEFAULT: &str = "deps/default";
pub const DEPS_CYCLE: &str = "deps/cycle";
pub const STALE_FILE: &str = "stale/file";
pub const STALE_BRICKS: &str = "stale/bricks";
pub const BRICKS_UNMAPPED: &str = "bricks/unmapped";

/// Complexity metrics; the code of a finding is `complexity/<metric>`.
pub const METRICS: [&str; 6] = [
    "cognitive",
    "cyclomatic",
    "nesting",
    "paths",
    "params",
    "lines",
];

/// All codes, complexity ones expanded.
#[cfg(test)]
pub fn all() -> Vec<String> {
    let mut v: Vec<String> = [
        DEPS_DENY,
        DEPS_DEFAULT,
        DEPS_CYCLE,
        STALE_FILE,
        STALE_BRICKS,
        BRICKS_UNMAPPED,
    ]
    .iter()
    .map(|c| c.to_string())
    .collect();
    v.extend(METRICS.iter().map(|m| format!("complexity/{m}")));
    v
}
