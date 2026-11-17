//! Operations over the `brickie.toml` tree. `file` is always the physical config file the op
//! edits, relative to the root config's directory with `/` separators. Elements are named by
//! identity, never by position: a brick by name, a rule or link by [`RuleKey`], a complexity rule by
//! its `bricks` selector.

use serde::{Deserialize, Serialize};

/// What a new brick is. Stored in the op only: it fills in `members` / `path` when they are
/// not given and drives scaffolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrickKind {
    /// A crate: members default to `path:<name>/` (`path:crates/<name>/` in the root file).
    Crate,
    /// A module of a crate: members must name it (`<crate>::<module>`).
    Module,
    /// A composite group: `path` defaults to `<name>`, and `<name>/brickie.toml` is created.
    Group,
}

/// A `[[brick]]` table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrickSpec {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<BrickKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
}

/// A `[[rule]]` or `[[link]]` table (both have the same fields); `None` and empty fields are
/// omitted from the file. For a link, `to` is a sibling brick or `composite/child`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSpec {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub reason: String,
}

/// Which `[[rule]]` or `[[link]]` of a file an op means: `from` and `to` as written, narrowed
/// by `kinds` (compared as a set) and `scope` (absent in the file means `all`) when given. The
/// identity is `(file, from, to, kinds, scope)`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleKey {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// The fields `set_rule` / `set_link` change; `None` keeps a field. The other fields are the
/// identity: change them with `rm` and `add`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A `[[complexity]]` table. As the patch of `set_complexity`, `None` keeps a field, `0`
/// removes a metric and an empty string removes `scope`; `bricks` is
/// ignored there (it is the identity).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplexitySpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bricks: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cognitive: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cyclomatic: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nesting: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// One design operation. JSON: `{"op": "add_brick", "file": "brickie.toml", ...}`.
///
/// `set_brick` changes only the fields given; an empty `desc` or `path` removes the key.
/// `make_composite` sets `path` on the brick and creates `<dir>/brickie.toml` (relative to
/// `file`'s directory). Complexity rules are named by their `bricks` selector (`None`: `*`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    AddBrick {
        file: String,
        brick: BrickSpec,
    },
    RemoveBrick {
        file: String,
        name: String,
    },
    SetBrick {
        file: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        desc: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        members: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// `true` writes `independent = true`, `false` removes the key; allowed at any level.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        independent: Option<bool>,
    },
    RenameBrick {
        file: String,
        from: String,
        to: String,
    },
    /// Move brick `name` of `file` into another level: the level file `into_file` itself
    /// (`into` `None`), or the children of brick `into` declared in `into_file` (made a
    /// composite at `path`, default its name, when it is a leaf).
    MoveBrick {
        file: String,
        name: String,
        into_file: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        into: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    MakeComposite {
        file: String,
        name: String,
        path: String,
    },
    /// Undo a composite: its children (with their rules and complexity rules) are lifted into
    /// `file`, the brick stays as a leaf without `path`, and its level file is dropped from the
    /// design (the file itself is left on disk).
    Ungroup {
        file: String,
        name: String,
    },
    /// Export `brick` from the composite level in `file` (nothing when already exported).
    AddExport {
        file: String,
        brick: String,
    },
    /// Stop exporting `brick` from the composite level in `file`.
    RemoveExport {
        file: String,
        brick: String,
    },
    AddRule {
        file: String,
        rule: RuleSpec,
    },
    RemoveRule {
        file: String,
        key: RuleKey,
    },
    SetRule {
        file: String,
        key: RuleKey,
        set: RulePatch,
    },
    AddLink {
        file: String,
        link: RuleSpec,
    },
    RemoveLink {
        file: String,
        key: RuleKey,
    },
    SetLink {
        file: String,
        key: RuleKey,
        set: RulePatch,
    },
    AddComplexity {
        file: String,
        rule: ComplexitySpec,
    },
    RemoveComplexity {
        file: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bricks: Option<String>,
    },
    SetComplexity {
        file: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bricks: Option<String>,
        set: ComplexitySpec,
    },
    AddInclude {
        file: String,
        path: String,
    },
    RemoveInclude {
        file: String,
        path: String,
    },
    CreateFile {
        file: String,
    },
}

impl Op {
    /// The physical config file the op edits.
    pub fn file(&self) -> &str {
        match self {
            Op::AddBrick { file, .. }
            | Op::RemoveBrick { file, .. }
            | Op::SetBrick { file, .. }
            | Op::RenameBrick { file, .. }
            | Op::MoveBrick { file, .. }
            | Op::MakeComposite { file, .. }
            | Op::Ungroup { file, .. }
            | Op::AddExport { file, .. }
            | Op::RemoveExport { file, .. }
            | Op::AddRule { file, .. }
            | Op::RemoveRule { file, .. }
            | Op::SetRule { file, .. }
            | Op::AddLink { file, .. }
            | Op::RemoveLink { file, .. }
            | Op::SetLink { file, .. }
            | Op::AddComplexity { file, .. }
            | Op::RemoveComplexity { file, .. }
            | Op::SetComplexity { file, .. }
            | Op::AddInclude { file, .. }
            | Op::RemoveInclude { file, .. }
            | Op::CreateFile { file } => file,
        }
    }

    /// The snake_case name, as in JSON.
    pub fn name(&self) -> &'static str {
        match self {
            Op::AddBrick { .. } => "add_brick",
            Op::RemoveBrick { .. } => "remove_brick",
            Op::SetBrick { .. } => "set_brick",
            Op::RenameBrick { .. } => "rename_brick",
            Op::MoveBrick { .. } => "move_brick",
            Op::MakeComposite { .. } => "make_composite",
            Op::Ungroup { .. } => "ungroup",
            Op::AddExport { .. } => "add_export",
            Op::RemoveExport { .. } => "remove_export",
            Op::AddRule { .. } => "add_rule",
            Op::RemoveRule { .. } => "remove_rule",
            Op::SetRule { .. } => "set_rule",
            Op::AddLink { .. } => "add_link",
            Op::RemoveLink { .. } => "remove_link",
            Op::SetLink { .. } => "set_link",
            Op::AddComplexity { .. } => "add_complexity",
            Op::RemoveComplexity { .. } => "remove_complexity",
            Op::SetComplexity { .. } => "set_complexity",
            Op::AddInclude { .. } => "add_include",
            Op::RemoveInclude { .. } => "remove_include",
            Op::CreateFile { .. } => "create_file",
        }
    }
}
