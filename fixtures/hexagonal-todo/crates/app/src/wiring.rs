//! Adapter selection: memory by default, local storage with feature `storage`.

use std::rc::Rc;

use todo_application::ports::inbound::BoardService;
use todo_application::usecases::BoardInteractor;

#[cfg(not(feature = "storage"))]
mod adapters {
    use todo_adapter_memory::{InMemoryColumnRepository, InMemoryNoteRepository, SequentialIds};

    pub type Columns = InMemoryColumnRepository;
    pub type Notes = InMemoryNoteRepository;
    pub type Ids = SequentialIds;

    pub fn columns() -> Columns {
        InMemoryColumnRepository::new()
    }

    pub fn notes() -> Notes {
        InMemoryNoteRepository::new()
    }

    pub fn ids() -> Ids {
        SequentialIds::new()
    }
}

#[cfg(feature = "storage")]
mod adapters {
    use todo_adapter_memory::SequentialIds;
    use todo_adapter_storage::{Backend, LocalStorageBoardRepository};
    use todo_application::ports::outbound::{ColumnRepository, NoteRepository};

    pub type Columns = LocalStorageBoardRepository;
    pub type Notes = LocalStorageBoardRepository;
    pub type Ids = SequentialIds;

    #[cfg(not(target_arch = "wasm32"))]
    fn backend() -> Backend {
        Backend::File(std::env::temp_dir().join("todo-app-board.json"))
    }

    #[cfg(target_arch = "wasm32")]
    fn backend() -> Backend {
        Backend::Browser("todo-app-board".to_string())
    }

    pub fn columns() -> Columns {
        LocalStorageBoardRepository::new(backend())
    }

    pub fn notes() -> Notes {
        LocalStorageBoardRepository::new(backend())
    }

    /// Seeds ids after the stored maxima. If the store cannot be read the board starts
    /// empty, so ids restart at 1 and nothing stored is reachable to collide with.
    pub fn ids() -> Ids {
        let repo = LocalStorageBoardRepository::new(backend());
        let column_max = ColumnRepository::load_all(&repo)
            .map(|c| c.iter().map(|c| c.id.0).max().unwrap_or(0))
            .unwrap_or(0);
        let note_max = NoteRepository::load_all(&repo)
            .map(|n| n.iter().map(|n| n.id.0).max().unwrap_or(0))
            .unwrap_or(0);
        SequentialIds::starting_after(column_max, note_max)
    }
}

/// Builds the board service over the adapters selected at compile time.
pub fn build_service() -> Rc<dyn BoardService> {
    Rc::new(BoardInteractor::new(
        adapters::columns(),
        adapters::notes(),
        adapters::ids(),
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub mod smoke;
