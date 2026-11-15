//! `LocalStorageBoardRepository`: both repository ports over one JSON snapshot.

use todo_application::ports::outbound::{ColumnRepository, NoteRepository, RepoError};
use todo_domain::column::Column;
use todo_domain::note::Note;

use crate::backend::Backend;
use crate::records::{ColumnRecord, NoteRecord, Snapshot};

pub struct LocalStorageBoardRepository {
    backend: Backend,
}

impl LocalStorageBoardRepository {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    fn snapshot(&self) -> Result<Snapshot, RepoError> {
        match self.backend.read()? {
            None => Ok(Snapshot::default()),
            Some(text) => serde_json::from_str(&text).map_err(|e| RepoError(e.to_string())),
        }
    }

    fn store(&self, snapshot: &Snapshot) -> Result<(), RepoError> {
        let text = serde_json::to_string(snapshot).map_err(|e| RepoError(e.to_string()))?;
        self.backend.write(&text)
    }
}

impl ColumnRepository for LocalStorageBoardRepository {
    fn load_all(&self) -> Result<Vec<Column>, RepoError> {
        let records = self.snapshot()?.columns;
        records.into_iter().map(Column::try_from).collect()
    }

    fn save_all(&self, columns: &[Column]) -> Result<(), RepoError> {
        let mut snapshot = self.snapshot()?;
        snapshot.columns = columns.iter().map(ColumnRecord::from).collect();
        self.store(&snapshot)
    }
}

impl NoteRepository for LocalStorageBoardRepository {
    fn load_all(&self) -> Result<Vec<Note>, RepoError> {
        let records = self.snapshot()?.notes;
        records.into_iter().map(Note::try_from).collect()
    }

    fn save_all(&self, notes: &[Note]) -> Result<(), RepoError> {
        let mut snapshot = self.snapshot()?;
        snapshot.notes = notes.iter().map(NoteRecord::from).collect();
        self.store(&snapshot)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use todo_domain::column::{Color, ColumnTitle};
    use todo_domain::error::{ColumnId, NoteId};
    use todo_domain::note::{NoteBody, NoteTitle, Position};

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "todo-adapter-storage-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("board.json")
    }

    fn column(id: u64, is_close: bool) -> Column {
        Column::new(
            ColumnId(id),
            ColumnTitle::parse("Done").unwrap(),
            Color::parse("#00ff00").unwrap(),
            is_close,
            id as usize,
        )
    }

    #[test]
    fn missing_file_loads_empty() {
        let repo = LocalStorageBoardRepository::new(Backend::File(temp_file("missing")));
        assert!(ColumnRepository::load_all(&repo).unwrap().is_empty());
        assert!(NoteRepository::load_all(&repo).unwrap().is_empty());
    }

    #[test]
    fn round_trip_keeps_both_collections() {
        let path = temp_file("round-trip");
        let repo = LocalStorageBoardRepository::new(Backend::File(path.clone()));
        let columns = vec![column(1, false), column(2, true)];
        let note = Note::new(
            NoteId(7),
            NoteTitle::parse("Ship").unwrap(),
            NoteBody::parse("body").unwrap(),
            &columns[1],
            Position(3),
        );
        ColumnRepository::save_all(&repo, &columns).unwrap();
        NoteRepository::save_all(&repo, std::slice::from_ref(&note)).unwrap();

        let again = LocalStorageBoardRepository::new(Backend::File(path));
        assert_eq!(ColumnRepository::load_all(&again).unwrap(), columns);
        assert_eq!(NoteRepository::load_all(&again).unwrap(), vec![note]);
    }

    #[test]
    fn corrupt_file_is_an_error() {
        let path = temp_file("corrupt");
        std::fs::write(&path, "not json").unwrap();
        let repo = LocalStorageBoardRepository::new(Backend::File(path));
        assert!(ColumnRepository::load_all(&repo).is_err());
    }

    #[test]
    fn invalid_stored_values_are_rejected() {
        let path = temp_file("invalid");
        std::fs::write(
            &path,
            r##"{"columns":[{"id":1,"title":"","color":"#000000","is_close":false,"order":0}]}"##,
        )
        .unwrap();
        let repo = LocalStorageBoardRepository::new(Backend::File(path));
        assert!(ColumnRepository::load_all(&repo).is_err());
    }
}
