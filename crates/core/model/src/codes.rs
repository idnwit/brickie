//! Every diagnostic code brickie can emit. `docs/llms.txt` must document each of them (a test
//! in the `brickie` crate (`cli/llms.rs`) enforces it), so a new code is added here first.

/// A code edge between sibling bricks that no valid `[[link]]` covers.
pub const DEPS_UNLINKED: &str = "deps/unlinked";
/// A cycle among bricks of one level.
pub const DEPS_CYCLE: &str = "deps/cycle";
/// A `.rs` file or workspace crate changed, was added or deleted since `measure`.
pub const STALE_FILE: &str = "stale/file";
/// The cache was measured with other features or targets than `[measure]` asks for, or in
/// another directory.
pub const STALE_MEASURE: &str = "stale/measure";
/// A use of an item that is not in the target brick's interface.
pub const DEPS_PRIVATE: &str = "deps/private";
/// A module that no brick claims.
pub const BRICKS_UNMAPPED: &str = "bricks/unmapped";
/// An exported item no other brick uses.
pub const INTERFACE_UNUSED: &str = "interface/unused";
// Config errors (exit 2): the message starts with the code.
pub const CONFIG_MISSING: &str = "config/missing";
pub const CONFIG_INVALID: &str = "config/invalid";
pub const CONFIG_MEMBER_OUTSIDE_PARENT: &str = "config/member-outside-parent";
pub const CONFIG_OVERLAP: &str = "config/overlap";
pub const CONFIG_UNASSIGNED: &str = "config/unassigned";
/// A `// brickie: allow(..)` directive that has no reason, names an unknown code, or exempts an
/// item that is not unused anyway.
pub const CONFIG_STALE_ALLOW: &str = "config/stale-allow";
// The design (`brickie design`): the message starts with the code.
pub const DESIGN_PLANNED: &str = "design/planned";
pub const DESIGN_INVALID: &str = "design/invalid";
/// A `[[link]]` that no `[[rule]]` of its level allows; it blocks `design publish`.
pub const DESIGN_LINK_DENIED: &str = "design/link-denied";
/// A write based on an older version than the head (exit code 3).
pub const DESIGN_STALE: &str = "design/stale";
pub const DESIGN_NO_MATCH: &str = "design/no-match";
pub const DESIGN_EXISTS: &str = "design/exists";
/// `design publish` refused: the code does not match the draft's structure yet (exit code 4).
pub const DESIGN_BLOCKED: &str = "design/blocked";

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
        DEPS_UNLINKED,
        DEPS_CYCLE,
        DEPS_PRIVATE,
        STALE_FILE,
        STALE_MEASURE,
        BRICKS_UNMAPPED,
        INTERFACE_UNUSED,
        CONFIG_MISSING,
        CONFIG_INVALID,
        CONFIG_MEMBER_OUTSIDE_PARENT,
        CONFIG_OVERLAP,
        CONFIG_UNASSIGNED,
        CONFIG_STALE_ALLOW,
        DESIGN_PLANNED,
        DESIGN_INVALID,
        DESIGN_LINK_DENIED,
        DESIGN_STALE,
        DESIGN_NO_MATCH,
        DESIGN_EXISTS,
        DESIGN_BLOCKED,
    ]
    .iter()
    .map(|c| c.to_string())
    .collect();
    v.extend(METRICS.iter().map(|m| format!("complexity/{m}")));
    v
}
