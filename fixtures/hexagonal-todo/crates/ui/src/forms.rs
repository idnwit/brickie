//! Forms: column and note create/edit forms with input validation messages.

use std::rc::Rc;

use sycamore::prelude::*;
use todo_application::ports::inbound::{AppError, ColumnDraft, NoteDraft};

/// Submit handler: returns the service result so the form can show a validation message.
pub type Submit<T> = Rc<dyn Fn(T) -> Result<(), AppError>>;

/// ColumnForm.
pub fn column_form(
    initial: ColumnDraft,
    label: &'static str,
    on_submit: Submit<ColumnDraft>,
) -> View {
    let title = create_signal(initial.title.clone());
    let color = create_signal(initial.color.clone());
    let is_close = create_signal(initial.is_close);
    let error = create_signal(String::new());
    let submit = move |ev: sycamore::web::events::SubmitEvent| {
        ev.prevent_default();
        let draft = ColumnDraft {
            title: title.get_clone(),
            color: color.get_clone(),
            is_close: is_close.get(),
        };
        // Batched so the refresh bump re-renders only after this handler's own sets.
        batch(|| match on_submit(draft) {
            Ok(()) => {
                error.set(String::new());
                title.set(initial.title.clone());
            }
            Err(e) => error.set(e.to_string()),
        });
    };
    view! {
        form(class="column-form", on:submit=submit) {
            input(placeholder="Column title", bind:value=title)
            input(r#type="color", bind:value=color)
            label { input(r#type="checkbox", bind:checked=is_close) " closes notes" }
            button(r#type="submit") { (label) }
            span(class="error") { (error.get_clone()) }
        }
    }
}

/// NoteForm.
pub fn note_form(initial: NoteDraft, label: &'static str, on_submit: Submit<NoteDraft>) -> View {
    let title = create_signal(initial.title.clone());
    let body = create_signal(initial.body.clone());
    let error = create_signal(String::new());
    let submit = move |ev: sycamore::web::events::SubmitEvent| {
        ev.prevent_default();
        let draft = NoteDraft {
            title: title.get_clone(),
            body: body.get_clone(),
        };
        batch(|| match on_submit(draft) {
            Ok(()) => {
                error.set(String::new());
                title.set(initial.title.clone());
                body.set(initial.body.clone());
            }
            Err(e) => error.set(e.to_string()),
        });
    };
    view! {
        form(class="note-form", on:submit=submit) {
            input(placeholder="Note title", bind:value=title)
            textarea(placeholder="Body", bind:value=body)
            button(r#type="submit") { (label) }
            span(class="error") { (error.get_clone()) }
        }
    }
}
