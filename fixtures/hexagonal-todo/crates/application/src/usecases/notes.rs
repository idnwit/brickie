//! Note use cases: create, edit, move (drag and drop) and delete notes.

use todo_domain::board::Board;
use todo_domain::error::{ColumnId, NoteId};
use todo_domain::note::{Note, NoteBody, NoteTitle, Position};

use crate::ports::inbound::{AppError, NoteDraft};
use crate::ports::outbound::IdGenerator;

/// Creates the note at the end of the column (a position past the end appends).
pub fn create(
    board: &mut Board,
    ids: &dyn IdGenerator,
    column: ColumnId,
    draft: NoteDraft,
) -> Result<NoteId, AppError> {
    let title = NoteTitle::parse(&draft.title)?;
    let body = NoteBody::parse(&draft.body)?;
    let target = board
        .column(column)
        .ok_or(AppError::ColumnNotFound(column))?;
    let id = ids.next_note_id();
    let note = Note::new(id, title, body, target, Position(usize::MAX));
    board.add_note(note)?;
    Ok(id)
}

pub fn update(board: &mut Board, id: NoteId, draft: NoteDraft) -> Result<(), AppError> {
    let title = NoteTitle::parse(&draft.title)?;
    let body = NoteBody::parse(&draft.body)?;
    let note = board.note_mut(id).ok_or(AppError::NoteNotFound(id))?;
    note.edit(title, body);
    Ok(())
}

pub fn move_to(
    board: &mut Board,
    id: NoteId,
    to: ColumnId,
    position: usize,
) -> Result<(), AppError> {
    Ok(board.move_note(id, to, Position(position))?)
}

pub fn delete(board: &mut Board, id: NoteId) -> Result<(), AppError> {
    board.remove_note(id)?;
    Ok(())
}
