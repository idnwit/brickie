//! Notes: `NoteId`, `NoteTitle`, `NoteBody`, `Position` and `Note` (column, position, closed state).

use crate::column::Column;
use crate::error::{ColumnId, DomainError, NoteId};

const MAX_TITLE: usize = 200;
const MAX_BODY: usize = 5000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTitle(String);

impl NoteTitle {
    /// Trims the input; it must be non-empty and at most 200 chars.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        if trimmed.chars().count() > MAX_TITLE {
            return Err(DomainError::TitleTooLong);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteBody(String);

impl NoteBody {
    /// At most 5000 chars; may be empty.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        if raw.chars().count() > MAX_BODY {
            return Err(DomainError::BodyTooLong);
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub id: NoteId,
    pub title: NoteTitle,
    pub body: NoteBody,
    pub column: ColumnId,
    pub position: Position,
    pub closed: bool,
}

impl Note {
    /// Creates a note already placed in `column` at `position`.
    pub fn new(
        id: NoteId,
        title: NoteTitle,
        body: NoteBody,
        column: &Column,
        position: Position,
    ) -> Self {
        Self {
            id,
            title,
            body,
            column: column.id,
            position,
            closed: column.is_close,
        }
    }

    pub fn edit(&mut self, title: NoteTitle, body: NoteBody) {
        self.title = title;
        self.body = body;
    }

    /// Places the note and derives `closed` from the target column.
    pub fn place(&mut self, column: &Column, position: Position) {
        self.column = column.id;
        self.position = position;
        self.closed = column.is_close;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{Color, ColumnTitle};

    fn column(id: u64, is_close: bool) -> Column {
        Column::new(
            ColumnId(id),
            ColumnTitle::parse("c").unwrap(),
            Color::parse("#000000").unwrap(),
            is_close,
            0,
        )
    }

    fn note(column: &Column) -> Note {
        Note::new(
            NoteId(1),
            NoteTitle::parse("t").unwrap(),
            NoteBody::parse("").unwrap(),
            column,
            Position(0),
        )
    }

    #[test]
    fn title_rules() {
        assert_eq!(NoteTitle::parse(" x ").unwrap().as_str(), "x");
        assert_eq!(NoteTitle::parse(""), Err(DomainError::EmptyTitle));
        assert_eq!(
            NoteTitle::parse(&"a".repeat(201)),
            Err(DomainError::TitleTooLong)
        );
        assert!(NoteTitle::parse(&"a".repeat(200)).is_ok());
    }

    #[test]
    fn body_rules() {
        assert!(NoteBody::parse("").is_ok());
        assert!(NoteBody::parse(&"a".repeat(5000)).is_ok());
        assert_eq!(
            NoteBody::parse(&"a".repeat(5001)),
            Err(DomainError::BodyTooLong)
        );
    }

    #[test]
    fn place_derives_closed() {
        let open = column(1, false);
        let done = column(2, true);
        let mut n = note(&open);
        assert!(!n.closed);
        n.place(&done, Position(3));
        assert!(n.closed);
        assert_eq!((n.column, n.position), (ColumnId(2), Position(3)));
        n.place(&open, Position(0));
        assert!(!n.closed);
    }

    #[test]
    fn edit_replaces_text() {
        let mut n = note(&column(1, false));
        n.edit(
            NoteTitle::parse("new").unwrap(),
            NoteBody::parse("b").unwrap(),
        );
        assert_eq!((n.title.as_str(), n.body.as_str()), ("new", "b"));
    }
}
