//! Every diagnostic code brickie can emit. `docs/llms.txt` must document each of them (a test
//! in the `brickie` crate (`cli/llms.rs`) enforces it), so a new code is added here first.

pub const DEPS_DENY: &str = "deps/deny";
pub const DEPS_DEFAULT: &str = "deps/default";
pub const DEPS_CYCLE: &str = "deps/cycle";
pub const STALE_FILE: &str = "stale/file";
pub const DEPS_PRIVATE: &str = "deps/private";
pub const BRICKS_UNMAPPED: &str = "bricks/unmapped";
pub const INTERFACE_UNUSED: &str = "interface/unused";
// Config errors (exit 2): the message starts with the code.
pub const CONFIG_MISSING: &str = "config/missing";
pub const CONFIG_MEMBER_OUTSIDE_PARENT: &str = "config/member-outside-parent";
pub const CONFIG_OVERLAP: &str = "config/overlap";
pub const CONFIG_UNASSIGNED: &str = "config/unassigned";
pub const CONFIG_UNKNOWN_EXPORT: &str = "config/unknown-export";
pub const CONFIG_UNKNOWN_VIA: &str = "config/unknown-via";
// The design (`brickie design`): the message starts with the code.
pub const DESIGN_PLANNED: &str = "design/planned";
pub const DESIGN_INVALID: &str = "design/invalid";
pub const DESIGN_PLANNED_EDGE: &str = "design/planned-edge";
pub const DESIGN_NO_REASON: &str = "design/no-reason";
/// A write based on an older version than the head (exit code 3).
pub const DESIGN_STALE: &str = "design/stale";
pub const DESIGN_NO_MATCH: &str = "design/no-match";
pub const DESIGN_EXISTS: &str = "design/exists";

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
pub fn all() -> Vec<String> {
    let mut v: Vec<String> = [
        DEPS_DENY,
        DEPS_DEFAULT,
        DEPS_CYCLE,
        DEPS_PRIVATE,
        STALE_FILE,
        BRICKS_UNMAPPED,
        INTERFACE_UNUSED,
        CONFIG_MISSING,
        CONFIG_MEMBER_OUTSIDE_PARENT,
        CONFIG_OVERLAP,
        CONFIG_UNASSIGNED,
        CONFIG_UNKNOWN_EXPORT,
        CONFIG_UNKNOWN_VIA,
        DESIGN_PLANNED,
        DESIGN_INVALID,
        DESIGN_PLANNED_EDGE,
        DESIGN_NO_REASON,
        DESIGN_STALE,
        DESIGN_NO_MATCH,
        DESIGN_EXISTS,
    ]
    .iter()
    .map(|c| c.to_string())
    .collect();
    v.extend(METRICS.iter().map(|m| format!("complexity/{m}")));
    v
}
