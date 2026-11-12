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
// Change sessions (`brickie change`, `check --change`).
pub const CHANGE_PLANNED: &str = "change/planned";
pub const CHANGE_CONFLICT: &str = "change/conflict";
pub const CHANGE_PROPOSED: &str = "change/proposed";
pub const CHANGE_OUTSIDE_EDIT: &str = "change/outside-edit";
pub const CHANGE_INVALID: &str = "change/invalid";
pub const CHANGE_OPEN: &str = "change/open";
pub const CHANGE_NONE: &str = "change/none";

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
        CHANGE_PLANNED,
        CHANGE_CONFLICT,
        CHANGE_PROPOSED,
        CHANGE_OUTSIDE_EDIT,
        CHANGE_INVALID,
        CHANGE_OPEN,
        CHANGE_NONE,
    ]
    .iter()
    .map(|c| c.to_string())
    .collect();
    v.extend(METRICS.iter().map(|m| format!("complexity/{m}")));
    v
}
