# Hexagonal Kanban: design

A kanban board in hexagonal (ports and adapters) style, with a Sycamore UI. It is the test bed for
brickie: the dev agent implements this design; the architecture agent owns every `brickie.toml`
and reviews against them. Crate package names keep the `todo-` prefix (no rename needed).

## Features

| Area | Feature |
|---|---|
| Columns | Create (title, color, `is_close`), list in board order, edit (title, color, `is_close`), reorder (move to index), delete |
| Notes | Create (title, body) in a column, edit title/body, delete, move to another column and/or position (drag and drop) |
| Closing | A note placed in an `is_close` column is closed; moved out of it, it is reopened. Toggling `is_close` on a column closes/reopens its notes |

## Domain rules (decisions)

| Rule | Decision |
|---|---|
| Note placement | A note belongs to exactly one existing column; it stores `ColumnId` and `Position` |
| Positions | Dense `0..n` per column; insert/move shifts the others; a target index beyond the end appends |
| Delete column with notes | Refused: `DomainError::ColumnNotEmpty`. Reason: no silent data loss, no hidden target choice. The UI asks the user to move notes first |
| Column order | Dense `0..n` across the board; delete compacts the order |
| Titles | Trimmed, non-empty, at most 100 chars (column) / 200 chars (note) |
| Body | At most 5000 chars, may be empty |
| Color | `#rrggbb` hex, parsed by `Color::parse` |
| Closed state | Derived on every placement: `closed = target_column.is_close` |

## Domain model (`todo-domain`)

| Module (brick) | Public items |
|---|---|
| `error` (`kernel`) | `struct ColumnId(u64)`; `struct NoteId(u64)`; `enum DomainError { EmptyTitle, TitleTooLong, BodyTooLong, InvalidColor, ColumnNotEmpty(ColumnId), UnknownColumn(ColumnId), UnknownNote(NoteId) }`. Ids live in the kernel so errors can name them without a `kernel -> column` cycle (decided after `brickie check` reported `deps/cycle`) |
| `column` | `struct ColumnTitle(String)` + `parse`; `struct Color(String)` + `parse`; `struct Column { id, title, color, is_close, order: usize }` with `new`, `rename`, `recolor`, `set_close` |
| `note` | `struct NoteTitle(String)` + `parse`; `struct NoteBody(String)` + `parse`; `struct Position(usize)`; `struct Note { id, title, body, column: ColumnId, position, closed }` with `new`, `edit`, `place(&Column, Position)` |
| `board` | `struct Board { columns: Vec<Column>, notes: Vec<Note> }` with `add_column`, `reorder_column(id, index)`, `remove_column(id) -> Result<Column, DomainError>`, `add_note`, `move_note(id, ColumnId, Position)`, `remove_note`, `set_column_close`; all invariants above live here |

`note` uses `column` (ids, `is_close`); `column` never uses `note`. No serde in the domain.

## Ports (`todo-application`)

```rust
// ports::inbound (board-service)
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
pub struct ColumnDraft { pub title: String, pub color: String, pub is_close: bool }
pub struct NoteDraft { pub title: String, pub body: String }
pub struct BoardView { pub columns: Vec<ColumnView> }            // ordered
pub struct ColumnView { pub id: ColumnId, pub title: String, pub color: String, pub is_close: bool, pub notes: Vec<NoteView> }
pub struct NoteView { pub id: NoteId, pub title: String, pub body: String, pub closed: bool }
pub enum AppError { Domain(DomainError), ColumnNotFound(ColumnId), NoteNotFound(NoteId), Storage(String) }

// ports::outbound (board-storage)
pub trait ColumnRepository { fn load_all(&self) -> Result<Vec<Column>, RepoError>; fn save_all(&self, columns: &[Column]) -> Result<(), RepoError>; }
pub trait NoteRepository   { fn load_all(&self) -> Result<Vec<Note>, RepoError>;   fn save_all(&self, notes: &[Note]) -> Result<(), RepoError>; }
pub trait IdGenerator      { fn next_column_id(&self) -> ColumnId; fn next_note_id(&self) -> NoteId; }
pub struct RepoError(pub String);
```

`save_all` writes a whole collection: moves renumber many positions, so per-item saves gain
nothing. No `Clock`: no feature needs time. Traits take `&self`; implementations use `RefCell`
(single-threaded WASM).

## Bricks

| Crate (path) | Module | Brick | Role |
|---|---|---|---|
| `todo-domain` (`crates/domain`) | whole crate | `domain` (composite, exports all four) | Domain layer |
| | root, `error` | `domain/kernel` | `DomainError` |
| | `column` | `domain/column` | Column entity and values |
| | `note` | `domain/note` | Note entity and values |
| | `board` | `domain/board` | Board aggregate, invariants |
| `todo-application` (`crates/application`) | whole crate | `application` (composite, exports `board-service`, `board-storage`) | Application layer |
| | `ports::inbound` | `application/board-service` | `BoardService` |
| | `ports::outbound` | `application/board-storage` | Repositories, `IdGenerator` |
| | root, `ports` (mod only), `usecases` (mod) | `application/usecases` | `BoardInteractor`, delegates |
| | `usecases::columns` | `application/usecases-columns` | Column use cases |
| | `usecases::notes` | `application/usecases-notes` | Note use cases |
| `todo-adapter-memory` (`crates/adapters/memory`) | all | `adapters/memory` | In-memory repos, `SequentialIds` |
| `todo-adapter-storage` (`crates/adapters/storage`) | all | `adapters/storage` | JSON local storage repo |
| `todo-ui` (`crates/ui`) | whole crate | `ui` (composite, exports `shell`) | Driving adapter |
| | root | `ui/shell` | `BoardApp`, `mount`, refresh |
| | `board` | `ui/board-view` | Column list, new-column form slot |
| | `column` | `ui/column-view` | Column header, note list, drop zone |
| | `note` | `ui/note-card` | Draggable card |
| | `drag` | `ui/drag` | `DragState`, `drop_target(..) -> Option<MoveNote>` (pure) |
| | `forms` | `ui/forms` | Column and note forms |
| `todo-app` (`crates/app`, bin) | all | `app` | Composition root |

`ports/mod.rs` and `usecases/mod.rs` only declare submodules (plus `BoardInteractor` in `usecases`).

## Allowed dependencies

Default deny, cycles deny, at every level. Anything not listed is an error.

| Level | From | May use | Kinds / `via` |
|---|---|---|---|
| root | `application` | `domain` | all |
| root | `adapters` | `application` | `via ["board-storage"]` |
| root | `adapters` | `domain` | all |
| root | `ui` | `application` | `via ["board-service"]` |
| root | `ui` | `domain` | `uses` only |
| root | `app` | `application`, `adapters`, `ui`, `domain` | all (explicit rules, so `deps/unused` flags a link that stops being used) |
| root | `tag:core` | `tag:adapter` | deny |
| domain | `column`, `note` | `kernel` | all |
| domain | `note` | `column` | all |
| domain | `board`, `kernel` | `*` | all |
| application | `usecases` | `*` | all |
| application | `usecases-columns`, `usecases-notes` | `board-service`, `board-storage` | all |
| ui | `shell` | `*` | all |
| ui | `board-view` | `column-view`, `forms`, `drag` | all |
| ui | `column-view` | `note-card`, `drag`, `forms` | all |
| ui | `note-card` | `drag`, `forms` | all |

Consequences: `column` never sees `note`/`board`; the two feature use-case modules never use each
other; `drag` and `forms` are leaves; only `shell` is visible to `app`; `application` also exports `usecases` so `app` can build `BoardInteractor`.

## Build targets

`brickie.toml` sets `[measure] additional_targets = ["wasm32-unknown-unknown"]`: the `app -> ui` edge (`todo_ui::mount`) exists only under `cfg(target_arch = "wasm32")`, so it is measured by the extra wasm32 run (all cargo features are always on).

- Every crate builds natively (`cargo check`, `cargo test`). Browser-only code (`sycamore::render`,
  `web-sys` storage, drag events) goes behind `#[cfg(target_arch = "wasm32")]`.
- Browser: `trunk serve crates/app/index.html`.
- `app` native `main` wires memory adapters, creates two columns and a note, moves it, prints the board.
- Feature `storage` on `todo-app` swaps in `LocalStorageBoardRepository` (file backend on native).

## Complexity budget

All bricks, prod code: cognitive <= 15, nesting <= 4, params <= 5 (errors). Tests not checked.
Keep logic outside `view!` (macro bodies are not measured).

## Dev tasks

| Crate | Tasks |
|---|---|
| `todo-domain` | 1. `error`: `DomainError`. 2. `column`: ids, `ColumnTitle`/`Color` parsing, `Column`. 3. `note`: values, `Note::place` sets `closed`. 4. `board`: every rule of the decision table, one method each. 5. Unit tests per rule (refused delete, close on move, reopen, position shifting, append past end). |
| `todo-application` | 1. `ports::inbound` and `ports::outbound` as signed above. 2. `usecases::columns`: free functions `create`, `update`, `reorder`, `delete` taking `&dyn ColumnRepository`, `&dyn NoteRepository`, `&dyn IdGenerator` as needed. 3. `usecases::notes`: `create`, `update`, `move_to`, `delete`. 4. `usecases::BoardInteractor<C, N, G>` implements `BoardService` by loading a `Board`, calling a feature function, saving. 5. Tests with fakes in test modules. |
| `todo-adapter-memory` | 1. `InMemoryColumnRepository`, `InMemoryNoteRepository` over `RefCell<Vec<_>>`. 2. `SequentialIds` (two counters). 3. Tests. |
| `todo-adapter-storage` | 1. Local serde record types (no serde in domain). 2. `LocalStorageBoardRepository` implementing both repository traits over one JSON snapshot; backend `web_sys::Storage` (wasm32) or a file (native). 3. Round-trip test (native, temp dir). |
| `todo-ui` | 1. `drag`: `DragState` signal and pure `drop_target(dragged, column, index) -> Option<MoveNote>` with tests. 2. `forms`: `ColumnForm`, `NoteForm` with validation messages from `AppError`. 3. `note`: `NoteCard` (draggable, edit, delete). 4. `column`: `ColumnView` (drop zone per gap, header actions, delete error shown). 5. `board`: `BoardView`. 6. root: `BoardApp(service: Rc<dyn BoardService>)`, refresh signal after every mutation, `mount` (wasm32). |
| `todo-app` | 1. `build_service` picks adapters (feature `storage`). 2. wasm32 `main` mounts; native smoke run. 3. `index.html`. |

## Architecture review tasks

| # | Check |
|---|---|
| 1 | `brickie check` exit 0; no `deps/*` finding, no complexity finding |
| 2 | `brickie interface domain/column`: no `note` or `board` type in signatures |
| 3 | `brickie interface application/board-service`: no `board-storage` type; `ui` imports only `board-service` and domain value types |
| 4 | Negative probes (revert after): `ui -> ports::outbound`, `adapters -> ports::inbound`, `usecases::columns -> usecases::notes`, `column -> note`, `drag -> note` must each fail `brickie check` |
| 5 | Board invariants are only in `domain/board`; use cases contain no placement arithmetic |
| 6 | No serde derive in `todo-domain`; no `web-sys` outside `adapters/storage` and `ui` |

## Verification

From this folder: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo build -p todo-app --target wasm32-unknown-unknown`, then
`brickie measure` and `brickie check` (exit 0).
