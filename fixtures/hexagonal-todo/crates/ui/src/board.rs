//! Board view: renders the ordered columns and the new-column form.

use std::rc::Rc;

use sycamore::prelude::*;
use todo_application::ports::inbound::{BoardView as BoardData, ColumnDraft};

use crate::column::column_view;
use crate::drag::Ctx;
use crate::forms::{Submit, column_form};

fn create_column(ctx: &Ctx) -> Submit<ColumnDraft> {
    let ctx = ctx.clone();
    Rc::new(move |draft| ctx.done(ctx.service.create_column(draft)))
}

fn column_list(board: &BoardData, ctx: &Ctx) -> View {
    let count = board.columns.len();
    View::from(
        board
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| column_view(c, i, count, ctx))
            .collect::<Vec<_>>(),
    )
}

/// The columns in order, followed by the new-column form.
pub fn board_view(board: &BoardData, ctx: &Ctx) -> View {
    let (board, ctx) = (board.clone(), ctx.clone());
    let initial = ColumnDraft {
        title: String::new(),
        color: "#4a90d9".to_string(),
        is_close: false,
    };
    let on_create = create_column(&ctx);
    view! {
        div(class="board") {
            (column_list(&board, &ctx))
            (column_form(initial.clone(), "Add column", on_create.clone()))
        }
    }
}
