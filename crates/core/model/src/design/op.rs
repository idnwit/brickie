//! Operations over the `brickie.toml` tree. `file` is always the physical config file the op
//! edits, relative to the root config's directory with `/` separators. Elements are named by
//! identity, never by position: a brick by name, a rule by [`RuleKey`], a complexity rule by
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

/// An `exports` entry: a plain name (`scope` `None`) or `{ brick, scope }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportSpec {
    pub brick: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// A `[[deps.rule]]` table; `None` and empty fields are omitted from the file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSpec {
    pub from: String,
    pub to: String,
    /// Exported children of the `to` composite; empty: any entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub via: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

/// Which `[[deps.rule]]` of a file an op means: `from` and `to` as written, narrowed by
/// `kinds` (compared as a set) and `scope` (absent in the file means `prod`) when given. A rule's
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

/// The fields `set_rule` changes; `None` keeps a field, an empty `reason` / `help` / `via`
/// removes it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

/// A `[[complexity]]` table. As the patch of `set_complexity`, `None` keeps a field, `0`
/// removes a metric and an empty string removes `level`, `scope` or `help`; `bricks` is
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
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
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
    SetExports {
        file: String,
        exports: Vec<ExportSpec>,
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
    SetDeps {
        file: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cycles: Option<String>,
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
            | Op::SetExports { file, .. }
            | Op::AddRule { file, .. }
            | Op::RemoveRule { file, .. }
            | Op::SetRule { file, .. }
            | Op::AddComplexity { file, .. }
            | Op::RemoveComplexity { file, .. }
            | Op::SetComplexity { file, .. }
            | Op::SetDeps { file, .. }
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
            Op::SetExports { .. } => "set_exports",
            Op::AddRule { .. } => "add_rule",
            Op::RemoveRule { .. } => "remove_rule",
            Op::SetRule { .. } => "set_rule",
            Op::AddComplexity { .. } => "add_complexity",
            Op::RemoveComplexity { .. } => "remove_complexity",
            Op::SetComplexity { .. } => "set_complexity",
            Op::SetDeps { .. } => "set_deps",
            Op::AddInclude { .. } => "add_include",
            Op::RemoveInclude { .. } => "remove_include",
            Op::CreateFile { .. } => "create_file",
        }
    }
}
