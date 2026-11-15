//! Shared domain error.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteId(pub u64);

/// Every way a domain rule can refuse an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    EmptyTitle,
    TitleTooLong,
    BodyTooLong,
    InvalidColor,
    ColumnNotEmpty(ColumnId),
    UnknownColumn(ColumnId),
    UnknownNote(NoteId),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTitle => write!(f, "title must not be empty"),
            Self::TitleTooLong => write!(f, "title is too long"),
            Self::BodyTooLong => write!(f, "body is too long"),
            Self::InvalidColor => write!(f, "color must be #rrggbb"),
            Self::ColumnNotEmpty(id) => write!(f, "column {} still has notes", id.0),
            Self::UnknownColumn(id) => write!(f, "unknown column {}", id.0),
            Self::UnknownNote(id) => write!(f, "unknown note {}", id.0),
        }
    }
}

impl std::error::Error for DomainError {}
