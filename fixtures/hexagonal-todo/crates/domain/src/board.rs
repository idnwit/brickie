//! Board aggregate: column order, note placement and closing, refused column deletion.

use crate::column::Column;
use crate::error::{ColumnId, DomainError, NoteId};
use crate::note::{Note, Position};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Board {
    pub columns: Vec<Column>,
    pub notes: Vec<Note>,
}

impl Board {
    pub fn new(columns: Vec<Column>, notes: Vec<Note>) -> Self {
        Self { columns, notes }
    }

    pub fn column(&self, id: ColumnId) -> Option<&Column> {
        self.columns.iter().find(|c| c.id == id)
    }

    pub fn column_mut(&mut self, id: ColumnId) -> Option<&mut Column> {
        self.columns.iter_mut().find(|c| c.id == id)
    }

    pub fn note(&self, id: NoteId) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn note_mut(&mut self, id: NoteId) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    /// Appends the column at the end of the board order.
    pub fn add_column(&mut self, mut column: Column) {
        column.order = self.columns.len();
        self.columns.push(column);
    }

    /// Moves the column to `index` (past the end appends) and keeps the order dense.
    pub fn reorder_column(&mut self, id: ColumnId, index: usize) -> Result<(), DomainError> {
        let from = self.column_index(id)?;
        let column = self.columns.remove(from);
        self.columns.insert(index.min(self.columns.len()), column);
        self.renumber_columns();
        Ok(())
    }

    /// Refused while the column still holds notes.
    pub fn remove_column(&mut self, id: ColumnId) -> Result<Column, DomainError> {
        let at = self.column_index(id)?;
        if self.notes.iter().any(|n| n.column == id) {
            return Err(DomainError::ColumnNotEmpty(id));
        }
        let column = self.columns.remove(at);
        self.renumber_columns();
        Ok(column)
    }

    /// Inserts the note into its column at its position (past the end appends).
    pub fn add_note(&mut self, note: Note) -> Result<(), DomainError> {
        let (column, position) = (note.column, note.position);
        self.insert_note(note, column, position)
    }

    /// Moves a note to another column and/or position.
    pub fn move_note(
        &mut self,
        id: NoteId,
        to: ColumnId,
        position: Position,
    ) -> Result<(), DomainError> {
        self.column_index(to)?;
        let note = self.remove_note(id)?;
        self.insert_note(note, to, position)
    }

    /// Removes the note and compacts the positions of its column.
    pub fn remove_note(&mut self, id: NoteId) -> Result<Note, DomainError> {
        let at = self
            .notes
            .iter()
            .position(|n| n.id == id)
            .ok_or(DomainError::UnknownNote(id))?;
        let note = self.notes.remove(at);
        self.renumber_notes(note.column);
        Ok(note)
    }

    /// Changes `is_close` and closes or reopens the notes of that column.
    pub fn set_column_close(&mut self, id: ColumnId, is_close: bool) -> Result<(), DomainError> {
        let at = self.column_index(id)?;
        self.columns[at].set_close(is_close);
        for note in self.notes.iter_mut().filter(|n| n.column == id) {
            note.closed = is_close;
        }
        Ok(())
    }

    fn column_index(&self, id: ColumnId) -> Result<usize, DomainError> {
        self.columns
            .iter()
            .position(|c| c.id == id)
            .ok_or(DomainError::UnknownColumn(id))
    }

    fn renumber_columns(&mut self) {
        for (order, column) in self.columns.iter_mut().enumerate() {
            column.order = order;
        }
    }

    /// Rewrites positions of a column's notes as dense `0..n`, keeping their relative order.
    fn renumber_notes(&mut self, column: ColumnId) {
        let mut in_column: Vec<&mut Note> = self
            .notes
            .iter_mut()
            .filter(|n| n.column == column)
            .collect();
        in_column.sort_by_key(|n| n.position);
        for (index, note) in in_column.into_iter().enumerate() {
            note.position = Position(index);
        }
    }

    fn insert_note(
        &mut self,
        mut note: Note,
        to: ColumnId,
        position: Position,
    ) -> Result<(), DomainError> {
        let at = self.column_index(to)?;
        let count = self.notes.iter().filter(|n| n.column == to).count();
        let slot = Position(position.0.min(count));
        for other in self
            .notes
            .iter_mut()
            .filter(|n| n.column == to && n.position >= slot)
        {
            other.position = Position(other.position.0 + 1);
        }
        note.place(&self.columns[at], slot);
        self.notes.push(note);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{Color, ColumnTitle};
    use crate::note::{NoteBody, NoteTitle};

    fn column(id: u64, is_close: bool) -> Column {
        Column::new(
            ColumnId(id),
            ColumnTitle::parse("c").unwrap(),
            Color::parse("#112233").unwrap(),
            is_close,
            0,
        )
    }

    fn board() -> Board {
        let mut b = Board::default();
        b.add_column(column(1, false));
        b.add_column(column(2, true));
        b.add_column(column(3, false));
        b
    }

    fn add(b: &mut Board, id: u64, col: u64, pos: usize) {
        let c = b.column(ColumnId(col)).unwrap().clone();
        let n = Note::new(
            NoteId(id),
            NoteTitle::parse("n").unwrap(),
            NoteBody::parse("").unwrap(),
            &c,
            Position(pos),
        );
        b.add_note(n).unwrap();
    }

    /// Note ids of a column in position order, asserting positions are dense.
    fn ids(b: &Board, col: u64) -> Vec<u64> {
        let mut v: Vec<&Note> = b
            .notes
            .iter()
            .filter(|n| n.column == ColumnId(col))
            .collect();
        v.sort_by_key(|n| n.position);
        for (i, n) in v.iter().enumerate() {
            assert_eq!(n.position, Position(i));
        }
        v.iter().map(|n| n.id.0).collect()
    }

    fn order(b: &Board) -> Vec<(u64, usize)> {
        b.columns.iter().map(|c| (c.id.0, c.order)).collect()
    }

    #[test]
    fn add_column_appends_in_order() {
        assert_eq!(order(&board()), [(1, 0), (2, 1), (3, 2)]);
    }

    #[test]
    fn reorder_column_keeps_dense_order() {
        let mut b = board();
        b.reorder_column(ColumnId(3), 0).unwrap();
        assert_eq!(order(&b), [(3, 0), (1, 1), (2, 2)]);
        b.reorder_column(ColumnId(3), 99).unwrap();
        assert_eq!(order(&b), [(1, 0), (2, 1), (3, 2)]);
        assert_eq!(
            b.reorder_column(ColumnId(9), 0),
            Err(DomainError::UnknownColumn(ColumnId(9)))
        );
    }

    #[test]
    fn delete_column_with_notes_is_refused() {
        let mut b = board();
        add(&mut b, 1, 1, 0);
        assert_eq!(
            b.remove_column(ColumnId(1)),
            Err(DomainError::ColumnNotEmpty(ColumnId(1)))
        );
        assert_eq!(b.columns.len(), 3);
    }

    #[test]
    fn delete_empty_column_compacts_order() {
        let mut b = board();
        let removed = b.remove_column(ColumnId(1)).unwrap();
        assert_eq!(removed.id, ColumnId(1));
        assert_eq!(order(&b), [(2, 0), (3, 1)]);
        assert_eq!(
            b.remove_column(ColumnId(1)),
            Err(DomainError::UnknownColumn(ColumnId(1)))
        );
    }

    #[test]
    fn add_note_shifts_following_notes() {
        let mut b = board();
        add(&mut b, 1, 1, 0);
        add(&mut b, 2, 1, 1);
        add(&mut b, 3, 1, 0);
        assert_eq!(ids(&b, 1), [3, 1, 2]);
    }

    #[test]
    fn add_note_past_end_appends() {
        let mut b = board();
        add(&mut b, 1, 1, 0);
        add(&mut b, 2, 1, 50);
        assert_eq!(ids(&b, 1), [1, 2]);
    }

    #[test]
    fn add_note_to_unknown_column_fails() {
        let mut b = board();
        let c = column(9, false);
        let n = Note::new(
            NoteId(1),
            NoteTitle::parse("n").unwrap(),
            NoteBody::parse("").unwrap(),
            &c,
            Position(0),
        );
        assert_eq!(b.add_note(n), Err(DomainError::UnknownColumn(ColumnId(9))));
    }

    #[test]
    fn move_within_column_shifts_positions() {
        let mut b = board();
        for i in 1..=3 {
            add(&mut b, i, 1, (i - 1) as usize);
        }
        assert_eq!(b.move_note(NoteId(3), ColumnId(1), Position(0)), Ok(()));
        assert_eq!(ids(&b, 1), [3, 1, 2]);
        assert_eq!(b.move_note(NoteId(3), ColumnId(1), Position(99)), Ok(()));
        assert_eq!(ids(&b, 1), [1, 2, 3]);
    }

    #[test]
    fn move_across_columns_compacts_source_and_shifts_target() {
        let mut b = board();
        add(&mut b, 1, 1, 0);
        add(&mut b, 2, 1, 1);
        add(&mut b, 3, 3, 0);
        b.move_note(NoteId(1), ColumnId(3), Position(0)).unwrap();
        assert_eq!(ids(&b, 1), [2]);
        assert_eq!(ids(&b, 3), [1, 3]);
    }

    #[test]
    fn move_into_close_column_closes_and_out_reopens() {
        let mut b = board();
        add(&mut b, 1, 1, 0);
        b.move_note(NoteId(1), ColumnId(2), Position(0)).unwrap();
        assert!(b.note(NoteId(1)).unwrap().closed);
        b.move_note(NoteId(1), ColumnId(1), Position(0)).unwrap();
        assert!(!b.note(NoteId(1)).unwrap().closed);
    }

    #[test]
    fn move_note_reports_unknown_targets() {
        let mut b = board();
        assert_eq!(
            b.move_note(NoteId(1), ColumnId(1), Position(0)),
            Err(DomainError::UnknownNote(NoteId(1)))
        );
        add(&mut b, 1, 1, 0);
        assert_eq!(
            b.move_note(NoteId(1), ColumnId(9), Position(0)),
            Err(DomainError::UnknownColumn(ColumnId(9)))
        );
        assert_eq!(ids(&b, 1), [1]);
    }

    #[test]
    fn remove_note_compacts_positions() {
        let mut b = board();
        for i in 1..=3 {
            add(&mut b, i, 1, (i - 1) as usize);
        }
        assert_eq!(b.remove_note(NoteId(2)).unwrap().id, NoteId(2));
        assert_eq!(ids(&b, 1), [1, 3]);
        assert_eq!(
            b.remove_note(NoteId(2)),
            Err(DomainError::UnknownNote(NoteId(2)))
        );
    }

    #[test]
    fn toggling_close_closes_and_reopens_notes() {
        let mut b = board();
        add(&mut b, 1, 1, 0);
        add(&mut b, 2, 3, 0);
        b.set_column_close(ColumnId(1), true).unwrap();
        assert!(b.note(NoteId(1)).unwrap().closed);
        assert!(!b.note(NoteId(2)).unwrap().closed);
        b.set_column_close(ColumnId(1), false).unwrap();
        assert!(!b.note(NoteId(1)).unwrap().closed);
        assert_eq!(
            b.set_column_close(ColumnId(9), true),
            Err(DomainError::UnknownColumn(ColumnId(9)))
        );
    }

    #[test]
    fn accessors_find_and_edit_entities() {
        let mut b = Board::new(board().columns, Vec::new());
        add(&mut b, 1, 1, 0);
        b.column_mut(ColumnId(1))
            .unwrap()
            .rename(ColumnTitle::parse("x").unwrap());
        assert_eq!(b.column(ColumnId(1)).unwrap().title.as_str(), "x");
        b.note_mut(NoteId(1)).unwrap().edit(
            NoteTitle::parse("y").unwrap(),
            NoteBody::parse("z").unwrap(),
        );
        assert_eq!(b.note(NoteId(1)).unwrap().body.as_str(), "z");
        assert!(b.column_mut(ColumnId(9)).is_none() && b.note_mut(NoteId(9)).is_none());
    }
}
