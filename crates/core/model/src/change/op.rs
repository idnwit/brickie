//! Operations over the `brickie.toml` tree. `file` is always the physical config file the op
//! edits, relative to the root config's directory with `/` separators.

use serde::{Deserialize, Serialize};

/// What a drawn brick is (the editor's palette). Stored in the op only: it fills in `members`
/// / `path` when they are not given and drives scaffolding on publish.
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

/// A `[[complexity]]` table.
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
/// `file`'s directory). Rule and complexity `index` count within `file`.
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
    /// composite at `path`, default its name, when it is a leaf). `Change::move_op` builds it
    /// from brick paths.
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
    /// design (publish lists it for deletion by hand).
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
        index: usize,
    },
    SetRule {
        file: String,
        index: usize,
        rule: RuleSpec,
    },
    AddComplexity {
        file: String,
        rule: ComplexitySpec,
    },
    RemoveComplexity {
        file: String,
        index: usize,
    },
    SetComplexity {
        file: String,
        index: usize,
        rule: ComplexitySpec,
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
