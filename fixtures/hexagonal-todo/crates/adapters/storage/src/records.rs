//! Private serde record types; the domain has no serde, so these convert to and from domain types.

use serde::{Deserialize, Serialize};
use todo_application::ports::outbound::RepoError;
use todo_domain::column::{Color, Column, ColumnTitle};
use todo_domain::error::{ColumnId, DomainError, NoteId};
use todo_domain::note::{Note, NoteBody, NoteTitle, Position};

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct Snapshot {
    #[serde(default)]
    pub columns: Vec<ColumnRecord>,
    #[serde(default)]
    pub notes: Vec<NoteRecord>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ColumnRecord {
    id: u64,
    title: String,
    color: String,
    is_close: bool,
    order: usize,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct NoteRecord {
    id: u64,
    title: String,
    body: String,
    column: u64,
    position: usize,
    closed: bool,
}

fn invalid(err: DomainError) -> RepoError {
    RepoError(format!("stored data is invalid: {err:?}"))
}

impl From<&Column> for ColumnRecord {
    fn from(c: &Column) -> Self {
        Self {
            id: c.id.0,
            title: c.title.as_str().to_string(),
            color: c.color.as_str().to_string(),
            is_close: c.is_close,
            order: c.order,
        }
    }
}

impl TryFrom<ColumnRecord> for Column {
    type Error = RepoError;

    fn try_from(r: ColumnRecord) -> Result<Self, RepoError> {
        let title = ColumnTitle::parse(&r.title).map_err(invalid)?;
        let color = Color::parse(&r.color).map_err(invalid)?;
        Ok(Column::new(
            ColumnId(r.id),
            title,
            color,
            r.is_close,
            r.order,
        ))
    }
}

impl From<&Note> for NoteRecord {
    fn from(n: &Note) -> Self {
        Self {
            id: n.id.0,
            title: n.title.as_str().to_string(),
            body: n.body.as_str().to_string(),
            column: n.column.0,
            position: n.position.0,
            closed: n.closed,
        }
    }
}

impl TryFrom<NoteRecord> for Note {
    type Error = RepoError;

    fn try_from(r: NoteRecord) -> Result<Self, RepoError> {
        Ok(Note {
            id: NoteId(r.id),
            title: NoteTitle::parse(&r.title).map_err(invalid)?,
            body: NoteBody::parse(&r.body).map_err(invalid)?,
            column: ColumnId(r.column),
            position: Position(r.position),
            closed: r.closed,
        })
    }
}
