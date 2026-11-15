//! Driven ports: `ColumnRepository`, `NoteRepository` and `IdGenerator`, implemented by the adapters.

use todo_domain::column::Column;
use todo_domain::error::{ColumnId, NoteId};
use todo_domain::note::Note;

/// Failure reported by a repository implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoError(pub String);

pub trait ColumnRepository {
    fn load_all(&self) -> Result<Vec<Column>, RepoError>;
    fn save_all(&self, columns: &[Column]) -> Result<(), RepoError>;
}

pub trait NoteRepository {
    fn load_all(&self) -> Result<Vec<Note>, RepoError>;
    fn save_all(&self, notes: &[Note]) -> Result<(), RepoError>;
}

pub trait IdGenerator {
    fn next_column_id(&self) -> ColumnId;
    fn next_note_id(&self) -> NoteId;
}
