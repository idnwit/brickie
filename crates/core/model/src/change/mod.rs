//! Change sessions: the plain, wasm-safe types shared by the engine (`brickie-change`), the CLI
//! and the web api. An `Op` is what a writer asks for; an `OpRecord` is the logged op with the
//! canonical TOML fragments it edits and its review state. No I/O here.

mod edge;
mod op;
mod record;
mod report;

pub use edge::{LinkCheck, PlannedEdge};
pub use op::{BrickKind, BrickSpec, ComplexitySpec, ExportSpec, Op, RuleSpec};
pub use record::{By, Entry, OpRecord, Review, ReviewEntry, State, Verdict};
pub use report::{
    DiffEntry, DiffOp, LayoutHint, LinkOption, Plan, PlanAction, PlanFile, Status, Task, TaskKind,
};
