//! Read model: maps a `Board` to the `BoardView` served by the driving port.

use todo_domain::board::Board;
use todo_domain::column::Column;

use crate::ports::inbound::{BoardView, ColumnView, NoteView};

pub(super) fn board_view(board: &Board) -> BoardView {
    let mut columns: Vec<&Column> = board.columns.iter().collect();
    columns.sort_by_key(|c| c.order);
    BoardView {
        columns: columns.into_iter().map(|c| column_view(board, c)).collect(),
    }
}

fn column_view(board: &Board, column: &Column) -> ColumnView {
    let mut notes: Vec<_> = board
        .notes
        .iter()
        .filter(|n| n.column == column.id)
        .collect();
    notes.sort_by_key(|n| n.position);
    ColumnView {
        id: column.id,
        title: column.title.as_str().to_owned(),
        color: column.color.as_str().to_owned(),
        is_close: column.is_close,
        notes: notes
            .into_iter()
            .map(|n| NoteView {
                id: n.id,
                title: n.title.as_str().to_owned(),
                body: n.body.as_str().to_owned(),
                closed: n.closed,
            })
            .collect(),
    }
}
