//! The design: the plain, wasm-safe types shared by the engine (`brickie-change`), the CLI and
//! the web api. An [`Op`] is one edit; a [`Version`] is one entry of the design's
//! history. No I/O here.

mod op;
mod report;
mod sheet;
mod version;

pub use op::{BrickKind, BrickSpec, ComplexitySpec, ExportSpec, Op, RuleKey, RulePatch, RuleSpec};
pub use report::{
    BrickView, DesignView, LevelView, LimitView, PlanAction, PlanFile, RuleView, Task, TaskKind,
};
pub use sheet::{FileSheet, LinkCheck, LinkOption};
pub use version::{By, Version};
