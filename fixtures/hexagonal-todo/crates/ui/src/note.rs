//! Note card: a draggable note with edit and delete actions.

use std::rc::Rc;

use sycamore::prelude::*;
use todo_application::ports::inbound::{NoteDraft, NoteView};
use todo_domain::error::ColumnId;

use crate::drag::{Ctx, Dragged};
use crate::forms::{Submit, note_form};

fn edit_handler(ctx: &Ctx, note: &NoteView, editing: Signal<bool>) -> Submit<NoteDraft> {
    let (ctx, id) = (ctx.clone(), note.id);
    Rc::new(move |draft| {
        let result = ctx.done(ctx.service.update_note(id, draft));
        if result.is_ok() {
            editing.set(false);
        }
        result
    })
}

fn edit_pane(draft: NoteDraft, on_edit: Submit<NoteDraft>, editing: Signal<bool>) -> View {
    let cancel = view! { button(on:click=move |_| editing.set(false)) { "Cancel" } };
    View::from(vec![note_form(draft, "Save", on_edit), cancel])
}

/// A card for `note`, which sits at `index` of `column`.
pub fn note_card(note: &NoteView, column: ColumnId, index: usize, ctx: &Ctx) -> View {
    let editing = create_signal(false);
    let error = create_signal(String::new());
    let id = note.id;
    let draft = NoteDraft {
        title: note.title.clone(),
        body: note.body.clone(),
    };
    let on_edit = edit_handler(ctx, note, editing);
    let delete_ctx = ctx.clone();
    let on_delete = move |_| delete_ctx.report(delete_ctx.service.delete_note(id), error);
    let drag = ctx.drag;
    let on_dragstart = move |_| {
        drag.start(Dragged {
            note: id,
            column,
            index,
        })
    };
    let on_dragend = move |_| drag.clear();
    let class = if note.closed { "note closed" } else { "note" };
    let (title, body) = (note.title.clone(), note.body.clone());
    view! {
        div(class=class, draggable="true", on:dragstart=on_dragstart, on:dragend=on_dragend) {
            (if editing.get() {
                edit_pane(draft.clone(), on_edit.clone(), editing)
            } else {
                let (title, body, on_delete) = (title.clone(), body.clone(), on_delete.clone());
                view! {
                    strong { (title) }
                    p { (body) }
                    button(on:click=move |_| editing.set(true)) { "Edit" }
                    button(on:click=on_delete) { "Delete" }
                }
            })
            span(class="error") { (error.get_clone()) }
        }
    }
}
