//! Column use cases: create, update, reorder and delete columns.

use todo_domain::board::Board;
use todo_domain::column::{Color, Column, ColumnTitle};
use todo_domain::error::ColumnId;

use crate::ports::inbound::{AppError, ColumnDraft};
use crate::ports::outbound::IdGenerator;

pub fn create(
    board: &mut Board,
    ids: &dyn IdGenerator,
    draft: ColumnDraft,
) -> Result<ColumnId, AppError> {
    let title = ColumnTitle::parse(&draft.title)?;
    let color = Color::parse(&draft.color)?;
    let id = ids.next_column_id();
    board.add_column(Column::new(id, title, color, draft.is_close, 0));
    Ok(id)
}

pub fn update(board: &mut Board, id: ColumnId, draft: ColumnDraft) -> Result<(), AppError> {
    let title = ColumnTitle::parse(&draft.title)?;
    let color = Color::parse(&draft.color)?;
    let column = board.column_mut(id).ok_or(AppError::ColumnNotFound(id))?;
    column.rename(title);
    column.recolor(color);
    board.set_column_close(id, draft.is_close)?;
    Ok(())
}

pub fn reorder(board: &mut Board, id: ColumnId, index: usize) -> Result<(), AppError> {
    Ok(board.reorder_column(id, index)?)
}

pub fn delete(board: &mut Board, id: ColumnId) -> Result<(), AppError> {
    board.remove_column(id)?;
    Ok(())
}
