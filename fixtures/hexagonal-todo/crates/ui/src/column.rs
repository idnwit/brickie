//! Column view: header (title, color, close flag), note list and drop zone.

use std::rc::Rc;

use sycamore::prelude::*;
use todo_application::ports::inbound::{ColumnDraft, ColumnView as ColumnData, NoteDraft};
use todo_domain::error::ColumnId;

use crate::drag::Ctx;
use crate::forms::{Submit, column_form, note_form};
use crate::note::note_card;

fn allow_drop(ev: sycamore::web::events::DragEvent) {
    ev.prevent_default();
}

/// A gap at `index` of `column` where a dragged note can be dropped.
fn drop_zone(column: ColumnId, index: usize, ctx: &Ctx) -> View {
    let error = create_signal(String::new());
    let ctx = ctx.clone();
    let on_drop = move |ev: sycamore::web::events::DragEvent| {
        ev.prevent_default();
        if let Some(m) = ctx.drag.drop_into(column, index) {
            ctx.report(ctx.service.move_note(m.note, m.to, m.position), error);
        }
    };
    view! {
        div(class="drop-zone", on:dragover=allow_drop, on:drop=on_drop) {
            span(class="error") { (error.get_clone()) }
        }
    }
}

fn add_note(ctx: &Ctx, column: ColumnId) -> Submit<NoteDraft> {
    let ctx = ctx.clone();
    Rc::new(move |draft| ctx.done(ctx.service.create_note(column, draft)))
}

fn edit_column(ctx: &Ctx, id: ColumnId, editing: Signal<bool>) -> Submit<ColumnDraft> {
    let ctx = ctx.clone();
    Rc::new(move |draft| {
        let result = ctx.done(ctx.service.update_column(id, draft));
        if result.is_ok() {
            editing.set(false);
        }
        result
    })
}

fn note_list(column: &ColumnData, ctx: &Ctx) -> View {
    let mut items = vec![drop_zone(column.id, 0, ctx)];
    for (index, note) in column.notes.iter().enumerate() {
        items.push(note_card(note, column.id, index, ctx));
        items.push(drop_zone(column.id, index + 1, ctx));
    }
    View::from(items)
}

/// The index a column at `index` of `count` moves to when shifted by `delta`, if it can move.
pub fn shift_target(index: usize, count: usize, delta: isize) -> Option<usize> {
    let target = index.checked_add_signed(delta)?;
    (target < count && target != index).then_some(target)
}

/// A button that moves column `id` to `target`, disabled when there is no target.
fn move_button(
    label: &'static str,
    id: ColumnId,
    target: Option<usize>,
    ctx: &Ctx,
    error: Signal<String>,
) -> View {
    let ctx = ctx.clone();
    let on_click = move |_| {
        if let Some(to) = target {
            ctx.report(ctx.service.reorder_column(id, to), error);
        }
    };
    view! { button(disabled=target.is_none(), on:click=on_click) { (label) } }
}

/// A column with its header, notes, drop zones and the add-note form.
/// It sits at `index` of `count` columns.
pub fn column_view(column: &ColumnData, index: usize, count: usize, ctx: &Ctx) -> View {
    let editing = create_signal(false);
    let error = create_signal(String::new());
    let id = column.id;
    let draft = ColumnDraft {
        title: column.title.clone(),
        color: column.color.clone(),
        is_close: column.is_close,
    };
    let on_edit = edit_column(ctx, id, editing);
    let on_add = add_note(ctx, id);
    let (column, ctx) = (column.clone(), ctx.clone());
    let delete_ctx = ctx.clone();
    let left = move_button("←", id, shift_target(index, count, -1), &ctx, error);
    let right = move_button("→", id, shift_target(index, count, 1), &ctx, error);
    let on_delete = move |_| delete_ctx.report(delete_ctx.service.delete_column(id), error);
    let style = format!("border-top: 4px solid {}", column.color);
    let suffix = if column.is_close { " (closes)" } else { "" };
    let heading = format!("{}{suffix}", column.title);
    let new_note = NoteDraft {
        title: String::new(),
        body: String::new(),
    };
    view! {
        section(class="column", style=style) {
            header {
                h2 { (heading) }
                (left)
                (right)
                button(on:click=move |_| editing.set(!editing.get())) { "Edit" }
                button(on:click=on_delete) { "Delete" }
                span(class="error") { (error.get_clone()) }
            }
            (if editing.get() {
                column_form(draft.clone(), "Save", on_edit.clone())
            } else {
                view! {}
            })
            div(class="notes") { (note_list(&column, &ctx)) }
            (note_form(new_note.clone(), "Add note", on_add.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::shift_target;

    #[test]
    fn first_cannot_move_left() {
        assert_eq!(shift_target(0, 3, -1), None);
        assert_eq!(shift_target(0, 3, 1), Some(1));
    }

    #[test]
    fn last_cannot_move_right() {
        assert_eq!(shift_target(2, 3, 1), None);
        assert_eq!(shift_target(2, 3, -1), Some(1));
    }

    #[test]
    fn single_column_cannot_move() {
        assert_eq!(shift_target(0, 1, -1), None);
        assert_eq!(shift_target(0, 1, 1), None);
    }
}
