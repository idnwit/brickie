//! In-memory driven adapter: `InMemoryColumnRepository`, `InMemoryNoteRepository`, `SequentialIds`.

use std::cell::{Cell, RefCell};

use todo_application::ports::outbound::{ColumnRepository, IdGenerator, NoteRepository, RepoError};
use todo_domain::column::Column;
use todo_domain::error::{ColumnId, NoteId};
use todo_domain::note::Note;

#[derive(Default)]
pub struct InMemoryColumnRepository {
    items: RefCell<Vec<Column>>,
}

impl InMemoryColumnRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ColumnRepository for InMemoryColumnRepository {
    fn load_all(&self) -> Result<Vec<Column>, RepoError> {
        Ok(self.items.borrow().clone())
    }

    fn save_all(&self, columns: &[Column]) -> Result<(), RepoError> {
        *self.items.borrow_mut() = columns.to_vec();
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryNoteRepository {
    items: RefCell<Vec<Note>>,
}

impl InMemoryNoteRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl NoteRepository for InMemoryNoteRepository {
    fn load_all(&self) -> Result<Vec<Note>, RepoError> {
        Ok(self.items.borrow().clone())
    }

    fn save_all(&self, notes: &[Note]) -> Result<(), RepoError> {
        *self.items.borrow_mut() = notes.to_vec();
        Ok(())
    }
}

/// Hands out ids starting at 1, with an independent counter per entity kind.
#[derive(Default)]
pub struct SequentialIds {
    column: Cell<u64>,
    note: Cell<u64>,
}

impl SequentialIds {
    pub fn new() -> Self {
        Self::default()
    }

    /// Continues after the given highest existing ids, so restored data never collides.
    pub fn starting_after(column_max: u64, note_max: u64) -> Self {
        Self {
            column: Cell::new(column_max),
            note: Cell::new(note_max),
        }
    }
}

impl IdGenerator for SequentialIds {
    fn next_column_id(&self) -> ColumnId {
        self.column.set(self.column.get() + 1);
        ColumnId(self.column.get())
    }

    fn next_note_id(&self) -> NoteId {
        self.note.set(self.note.get() + 1);
        NoteId(self.note.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use todo_domain::column::{Color, ColumnTitle};
    use todo_domain::note::{NoteBody, NoteTitle, Position};

    fn column(id: u64) -> Column {
        Column::new(
            ColumnId(id),
            ColumnTitle::parse("c").unwrap(),
            Color::parse("#000000").unwrap(),
            false,
            0,
        )
    }

    #[test]
    fn empty_repositories_load_nothing() {
        assert!(
            InMemoryColumnRepository::new()
                .load_all()
                .unwrap()
                .is_empty()
        );
        assert!(InMemoryNoteRepository::new().load_all().unwrap().is_empty());
    }

    #[test]
    fn save_replaces_content() {
        let repo = InMemoryColumnRepository::new();
        repo.save_all(&[column(1), column(2)]).unwrap();
        repo.save_all(&[column(3)]).unwrap();
        assert_eq!(repo.load_all().unwrap(), vec![column(3)]);
    }

    #[test]
    fn notes_round_trip() {
        let repo = InMemoryNoteRepository::new();
        let note = Note::new(
            NoteId(1),
            NoteTitle::parse("t").unwrap(),
            NoteBody::parse("b").unwrap(),
            &column(1),
            Position(0),
        );
        repo.save_all(std::slice::from_ref(&note)).unwrap();
        assert_eq!(repo.load_all().unwrap(), vec![note]);
    }

    #[test]
    fn ids_are_sequential_per_kind() {
        let ids = SequentialIds::new();
        assert_eq!(ids.next_column_id(), ColumnId(1));
        assert_eq!(ids.next_column_id(), ColumnId(2));
        assert_eq!(ids.next_note_id(), NoteId(1));
        assert_eq!(ids.next_column_id(), ColumnId(3));
    }

    #[test]
    fn ids_continue_after_existing_maxima() {
        let ids = SequentialIds::starting_after(5, 9);
        assert_eq!(ids.next_column_id(), ColumnId(6));
        assert_eq!(ids.next_note_id(), NoteId(10));
    }
}
