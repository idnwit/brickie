//! Use cases: `BoardInteractor`, the implementation of `BoardService` over the driven ports.

pub mod columns;
pub mod notes;
mod view;

use todo_domain::board::Board;
use todo_domain::error::{ColumnId, NoteId};

use crate::ports::inbound::{AppError, BoardService, BoardView, ColumnDraft, NoteDraft};
use crate::ports::outbound::{ColumnRepository, IdGenerator, NoteRepository, RepoError};

impl From<RepoError> for AppError {
    fn from(error: RepoError) -> Self {
        Self::Storage(error.0)
    }
}

/// Loads a `Board`, runs one feature function on it and saves the result.
pub struct BoardInteractor<C, N, G> {
    columns: C,
    notes: N,
    ids: G,
}

impl<C: ColumnRepository, N: NoteRepository, G: IdGenerator> BoardInteractor<C, N, G> {
    pub fn new(columns: C, notes: N, ids: G) -> Self {
        Self {
            columns,
            notes,
            ids,
        }
    }

    fn load(&self) -> Result<Board, AppError> {
        Ok(Board::new(self.columns.load_all()?, self.notes.load_all()?))
    }

    /// Runs `change` on the loaded board; saves only when it succeeds.
    fn mutate<T>(
        &self,
        change: impl FnOnce(&mut Board, &G) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut board = self.load()?;
        let out = change(&mut board, &self.ids)?;
        self.columns.save_all(&board.columns)?;
        self.notes.save_all(&board.notes)?;
        Ok(out)
    }
}

impl<C: ColumnRepository, N: NoteRepository, G: IdGenerator> BoardService
    for BoardInteractor<C, N, G>
{
    fn board(&self) -> Result<BoardView, AppError> {
        Ok(view::board_view(&self.load()?))
    }

    fn create_column(&self, cmd: ColumnDraft) -> Result<ColumnId, AppError> {
        self.mutate(|b, ids| columns::create(b, ids, cmd))
    }

    fn update_column(&self, id: ColumnId, cmd: ColumnDraft) -> Result<(), AppError> {
        self.mutate(|b, _| columns::update(b, id, cmd))
    }

    fn reorder_column(&self, id: ColumnId, index: usize) -> Result<(), AppError> {
        self.mutate(|b, _| columns::reorder(b, id, index))
    }

    fn delete_column(&self, id: ColumnId) -> Result<(), AppError> {
        self.mutate(|b, _| columns::delete(b, id))
    }

    fn create_note(&self, column: ColumnId, cmd: NoteDraft) -> Result<NoteId, AppError> {
        self.mutate(|b, ids| notes::create(b, ids, column, cmd))
    }

    fn update_note(&self, id: NoteId, cmd: NoteDraft) -> Result<(), AppError> {
        self.mutate(|b, _| notes::update(b, id, cmd))
    }

    fn move_note(&self, id: NoteId, to: ColumnId, position: usize) -> Result<(), AppError> {
        self.mutate(|b, _| notes::move_to(b, id, to, position))
    }

    fn delete_note(&self, id: NoteId) -> Result<(), AppError> {
        self.mutate(|b, _| notes::delete(b, id))
    }
}

#[cfg(test)]
mod tests;
