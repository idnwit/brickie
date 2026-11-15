//! Drag state: the dragged note and drop target, and the pure drop-to-move computation.

use std::rc::Rc;

use sycamore::prelude::*;
use todo_application::ports::inbound::{AppError, BoardService};
use todo_domain::error::{ColumnId, NoteId};

/// The note being dragged and where it currently sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dragged {
    pub note: NoteId,
    pub column: ColumnId,
    pub index: usize,
}

/// A request to move a note; `position` is the index after the note left its old place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveNote {
    pub note: NoteId,
    pub to: ColumnId,
    pub position: usize,
}

/// Shared reactive drag state, created once by the shell.
#[derive(Clone, Copy)]
pub struct DragState {
    dragged: Signal<Option<Dragged>>,
}

impl DragState {
    pub fn new() -> Self {
        Self {
            dragged: create_signal(None),
        }
    }

    pub fn start(&self, dragged: Dragged) {
        self.dragged.set(Some(dragged));
    }

    pub fn clear(&self) {
        self.dragged.set(None);
    }

    pub fn current(&self) -> Option<Dragged> {
        self.dragged.get_untracked()
    }

    /// Ends the drag and returns the move for dropping into the gap `index` of `column`.
    pub fn drop_into(&self, column: ColumnId, index: usize) -> Option<MoveNote> {
        let target = drop_target(self.current(), column, index);
        self.clear();
        target
    }
}

impl Default for DragState {
    fn default() -> Self {
        Self::new()
    }
}

/// What every view needs: the service, the refresh signal and the drag state.
#[derive(Clone)]
pub struct Ctx {
    pub service: Rc<dyn BoardService>,
    pub refresh: Signal<u32>,
    pub drag: DragState,
}

impl Ctx {
    pub fn new(service: Rc<dyn BoardService>) -> Self {
        Self {
            service,
            refresh: create_signal(0),
            drag: DragState::new(),
        }
    }

    /// Reloads the board.
    fn bump(&self) {
        self.refresh.set(self.refresh.get_untracked() + 1);
    }

    /// Reloads the board after a success and drops the value, so forms can show the error.
    pub fn done<T>(&self, result: Result<T, AppError>) -> Result<(), AppError> {
        result.map(|_| self.bump())
    }

    /// Like `done`, but keeps the message in `error` for display next to the action.
    pub fn report<T>(&self, result: Result<T, AppError>, error: Signal<String>) {
        // Batched so the refresh bump re-renders only after `error` is set.
        batch(|| match self.done(result) {
            Ok(()) => error.set(String::new()),
            Err(e) => error.set(e.to_string()),
        });
    }
}

/// Computes the move for dropping `dragged` into the gap `index` of `column`.
/// Returns `None` when nothing is dragged or the drop would not change the order.
pub fn drop_target(dragged: Option<Dragged>, column: ColumnId, index: usize) -> Option<MoveNote> {
    let d = dragged?;
    if d.column != column {
        return Some(MoveNote {
            note: d.note,
            to: column,
            position: index,
        });
    }
    if index == d.index || index == d.index + 1 {
        return None;
    }
    let position = if index > d.index { index - 1 } else { index };
    Some(MoveNote {
        note: d.note,
        to: column,
        position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: ColumnId = ColumnId(1);
    const B: ColumnId = ColumnId(2);

    fn dragged(column: ColumnId, index: usize) -> Option<Dragged> {
        Some(Dragged {
            note: NoteId(7),
            column,
            index,
        })
    }

    fn mv(to: ColumnId, position: usize) -> Option<MoveNote> {
        Some(MoveNote {
            note: NoteId(7),
            to,
            position,
        })
    }

    #[test]
    fn nothing_dragged_gives_no_move() {
        assert_eq!(drop_target(None, A, 0), None);
    }

    #[test]
    fn other_column_keeps_gap_index() {
        assert_eq!(drop_target(dragged(A, 2), B, 0), mv(B, 0));
        assert_eq!(drop_target(dragged(A, 0), B, 3), mv(B, 3));
    }

    #[test]
    fn same_place_is_ignored() {
        assert_eq!(drop_target(dragged(A, 2), A, 2), None);
        assert_eq!(drop_target(dragged(A, 2), A, 3), None);
    }

    #[test]
    fn moving_down_accounts_for_removal() {
        assert_eq!(drop_target(dragged(A, 0), A, 3), mv(A, 2));
    }

    #[test]
    fn moving_up_keeps_gap_index() {
        assert_eq!(drop_target(dragged(A, 3), A, 1), mv(A, 1));
    }

    #[test]
    fn state_drop_clears_the_drag() {
        let root = create_root(|| {
            let state = DragState::new();
            state.start(Dragged {
                note: NoteId(7),
                column: A,
                index: 0,
            });
            assert_eq!(state.drop_into(B, 1), mv(B, 1));
            assert_eq!(state.current(), None);
            assert_eq!(state.drop_into(B, 1), None);
        });
        root.dispose();
    }
}
