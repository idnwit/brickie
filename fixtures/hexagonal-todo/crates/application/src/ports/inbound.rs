//! Driving port: the `BoardService` trait, its commands and read models used by the UI.

use std::fmt;

use todo_domain::error::{ColumnId, DomainError, NoteId};

pub trait BoardService {
    fn board(&self) -> Result<BoardView, AppError>;
    fn create_column(&self, cmd: ColumnDraft) -> Result<ColumnId, AppError>;
    fn update_column(&self, id: ColumnId, cmd: ColumnDraft) -> Result<(), AppError>;
    fn reorder_column(&self, id: ColumnId, index: usize) -> Result<(), AppError>;
    fn delete_column(&self, id: ColumnId) -> Result<(), AppError>;
    fn create_note(&self, column: ColumnId, cmd: NoteDraft) -> Result<NoteId, AppError>;
    fn update_note(&self, id: NoteId, cmd: NoteDraft) -> Result<(), AppError>;
    fn move_note(&self, id: NoteId, to: ColumnId, position: usize) -> Result<(), AppError>;
    fn delete_note(&self, id: NoteId) -> Result<(), AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDraft {
    pub title: String,
    pub color: String,
    pub is_close: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteDraft {
    pub title: String,
    pub body: String,
}

/// Columns in board order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardView {
    pub columns: Vec<ColumnView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnView {
    pub id: ColumnId,
    pub title: String,
    pub color: String,
    pub is_close: bool,
    pub notes: Vec<NoteView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteView {
    pub id: NoteId,
    pub title: String,
    pub body: String,
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Domain(DomainError),
    ColumnNotFound(ColumnId),
    NoteNotFound(NoteId),
    Storage(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(e) => e.fmt(f),
            Self::ColumnNotFound(id) => write!(f, "column {} not found", id.0),
            Self::NoteNotFound(id) => write!(f, "note {} not found", id.0),
            Self::Storage(msg) => write!(f, "storage error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<DomainError> for AppError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::UnknownColumn(id) => Self::ColumnNotFound(id),
            DomainError::UnknownNote(id) => Self::NoteNotFound(id),
            other => Self::Domain(other),
        }
    }
}
