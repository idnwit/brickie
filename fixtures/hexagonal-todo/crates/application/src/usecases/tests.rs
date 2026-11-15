use std::cell::{Cell, RefCell};

use todo_domain::column::Column;
use todo_domain::error::{ColumnId, DomainError, NoteId};
use todo_domain::note::Note;

use super::BoardInteractor;
use crate::ports::inbound::{AppError, BoardService, ColumnDraft, NoteDraft};
use crate::ports::outbound::{ColumnRepository, IdGenerator, NoteRepository, RepoError};

#[derive(Default)]
struct FakeColumns {
    items: RefCell<Vec<Column>>,
    fail: Cell<bool>,
}

impl ColumnRepository for &FakeColumns {
    fn load_all(&self) -> Result<Vec<Column>, RepoError> {
        if self.fail.get() {
            return Err(RepoError("disk".into()));
        }
        Ok(self.items.borrow().clone())
    }
    fn save_all(&self, columns: &[Column]) -> Result<(), RepoError> {
        *self.items.borrow_mut() = columns.to_vec();
        Ok(())
    }
}

#[derive(Default)]
struct FakeNotes {
    items: RefCell<Vec<Note>>,
}

impl NoteRepository for &FakeNotes {
    fn load_all(&self) -> Result<Vec<Note>, RepoError> {
        Ok(self.items.borrow().clone())
    }
    fn save_all(&self, notes: &[Note]) -> Result<(), RepoError> {
        *self.items.borrow_mut() = notes.to_vec();
        Ok(())
    }
}

#[derive(Default)]
struct FakeIds {
    next: Cell<u64>,
}

impl IdGenerator for &FakeIds {
    fn next_column_id(&self) -> ColumnId {
        self.next.set(self.next.get() + 1);
        ColumnId(self.next.get())
    }
    fn next_note_id(&self) -> NoteId {
        self.next.set(self.next.get() + 1);
        NoteId(self.next.get())
    }
}

struct Fixture {
    columns: FakeColumns,
    notes: FakeNotes,
    ids: FakeIds,
}

impl Fixture {
    fn new() -> Self {
        Self {
            columns: FakeColumns::default(),
            notes: FakeNotes::default(),
            ids: FakeIds::default(),
        }
    }

    fn service(&self) -> BoardInteractor<&FakeColumns, &FakeNotes, &FakeIds> {
        BoardInteractor::new(&self.columns, &self.notes, &self.ids)
    }
}

fn col(title: &str, is_close: bool) -> ColumnDraft {
    ColumnDraft {
        title: title.into(),
        color: "#112233".into(),
        is_close,
    }
}

fn note(title: &str) -> NoteDraft {
    NoteDraft {
        title: title.into(),
        body: "b".into(),
    }
}

#[test]
fn creates_columns_in_order_and_lists_them() {
    let fx = Fixture::new();
    let s = fx.service();
    let a = s.create_column(col("Todo", false)).unwrap();
    let b = s.create_column(col("Done", true)).unwrap();
    let view = s.board().unwrap();
    let got: Vec<ColumnId> = view.columns.iter().map(|c| c.id).collect();
    assert_eq!(got, [a, b]);
    assert!(view.columns[1].is_close);
}

#[test]
fn invalid_draft_is_a_domain_error_and_saves_nothing() {
    let fx = Fixture::new();
    let s = fx.service();
    assert_eq!(
        s.create_column(col("  ", false)),
        Err(AppError::Domain(DomainError::EmptyTitle))
    );
    let bad = ColumnDraft {
        title: "x".into(),
        color: "red".into(),
        is_close: false,
    };
    assert_eq!(
        s.create_column(bad),
        Err(AppError::Domain(DomainError::InvalidColor))
    );
    assert!(s.board().unwrap().columns.is_empty());
}

#[test]
fn update_and_reorder_column() {
    let fx = Fixture::new();
    let s = fx.service();
    let a = s.create_column(col("A", false)).unwrap();
    let b = s.create_column(col("B", false)).unwrap();
    s.update_column(a, col("A2", true)).unwrap();
    s.reorder_column(b, 0).unwrap();
    let view = s.board().unwrap();
    assert_eq!(view.columns[0].id, b);
    assert_eq!(view.columns[1].title, "A2");
    assert!(view.columns[1].is_close);
    assert_eq!(
        s.update_column(ColumnId(99), col("x", false)),
        Err(AppError::ColumnNotFound(ColumnId(99)))
    );
    assert_eq!(
        s.reorder_column(ColumnId(99), 0),
        Err(AppError::ColumnNotFound(ColumnId(99)))
    );
}

#[test]
fn delete_column_refused_while_it_has_notes() {
    let fx = Fixture::new();
    let s = fx.service();
    let c = s.create_column(col("A", false)).unwrap();
    let n = s.create_note(c, note("n")).unwrap();
    assert_eq!(
        s.delete_column(c),
        Err(AppError::Domain(DomainError::ColumnNotEmpty(c)))
    );
    s.delete_note(n).unwrap();
    s.delete_column(c).unwrap();
    assert!(s.board().unwrap().columns.is_empty());
}

#[test]
fn notes_append_edit_and_move() {
    let fx = Fixture::new();
    let s = fx.service();
    let todo = s.create_column(col("Todo", false)).unwrap();
    let done = s.create_column(col("Done", true)).unwrap();
    let n1 = s.create_note(todo, note("one")).unwrap();
    let n2 = s.create_note(todo, note("two")).unwrap();
    s.update_note(
        n1,
        NoteDraft {
            title: "uno".into(),
            body: "x".into(),
        },
    )
    .unwrap();
    s.move_note(n2, done, 0).unwrap();
    let view = s.board().unwrap();
    assert_eq!(view.columns[0].notes.len(), 1);
    assert_eq!(view.columns[0].notes[0].title, "uno");
    assert!(!view.columns[0].notes[0].closed);
    assert_eq!(view.columns[1].notes[0].id, n2);
    assert!(view.columns[1].notes[0].closed);
}

#[test]
fn note_errors_name_the_missing_entity() {
    let fx = Fixture::new();
    let s = fx.service();
    let c = s.create_column(col("A", false)).unwrap();
    assert_eq!(
        s.create_note(ColumnId(9), note("n")),
        Err(AppError::ColumnNotFound(ColumnId(9)))
    );
    assert_eq!(
        s.update_note(NoteId(9), note("n")),
        Err(AppError::NoteNotFound(NoteId(9)))
    );
    assert_eq!(
        s.move_note(NoteId(9), c, 0),
        Err(AppError::NoteNotFound(NoteId(9)))
    );
    assert_eq!(
        s.delete_note(NoteId(9)),
        Err(AppError::NoteNotFound(NoteId(9)))
    );
    assert_eq!(
        s.create_note(c, note("")),
        Err(AppError::Domain(DomainError::EmptyTitle))
    );
}

#[test]
fn storage_failure_maps_to_storage_error() {
    let fx = Fixture::new();
    fx.columns.fail.set(true);
    assert_eq!(fx.service().board(), Err(AppError::Storage("disk".into())));
}
