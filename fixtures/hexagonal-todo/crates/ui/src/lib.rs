//! Driving adapter: Sycamore kanban board (`BoardApp` and children) that calls the `BoardService` port.

use std::rc::Rc;

use sycamore::prelude::*;
use todo_application::ports::inbound::BoardService;

pub mod board;
pub mod column;
pub mod drag;
pub mod forms;
pub mod note;

use board::board_view;
use drag::Ctx;

fn render_board(ctx: &Ctx) -> View {
    match ctx.service.board() {
        Ok(board) => board_view(&board, ctx),
        Err(error) => {
            let message = format!("Cannot load the board: {error}");
            view! { p(class="error") { (message) } }
        }
    }
}

/// The whole board. Every successful mutation bumps the refresh signal, which reloads the board.
/// Call it inside a reactive root, for example `render(move || BoardApp(service))`.
#[allow(non_snake_case)]
pub fn BoardApp(service: Rc<dyn BoardService>) -> View {
    let ctx = Ctx::new(service);
    view! {
        main(class="app") {
            h1 { "Kanban" }
            (move || {
                ctx.refresh.track();
                render_board(&ctx)
            })
        }
    }
}

/// Mounts the app into the document body (browser only).
#[cfg(target_arch = "wasm32")]
pub fn mount(service: Rc<dyn BoardService>) {
    sycamore::render(move || BoardApp(service.clone()));
}

#[cfg(test)]
mod tests {
    use todo_application::ports::inbound::{
        AppError, BoardView as BoardData, ColumnDraft, ColumnView as ColumnData, NoteDraft,
    };
    use todo_domain::error::{ColumnId, NoteId};

    use sycamore::render_to_string;

    use super::*;

    struct OneColumn;

    impl BoardService for OneColumn {
        fn board(&self) -> Result<BoardData, AppError> {
            let column = ColumnData {
                id: ColumnId(1),
                title: "Todo".into(),
                color: "#112233".into(),
                is_close: false,
                notes: vec![],
            };
            Ok(BoardData {
                columns: vec![column],
            })
        }
        fn create_column(&self, _: ColumnDraft) -> Result<ColumnId, AppError> {
            Ok(ColumnId(2))
        }
        fn update_column(&self, _: ColumnId, _: ColumnDraft) -> Result<(), AppError> {
            Ok(())
        }
        fn reorder_column(&self, _: ColumnId, _: usize) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_column(&self, _: ColumnId) -> Result<(), AppError> {
            Ok(())
        }
        fn create_note(&self, _: ColumnId, _: NoteDraft) -> Result<NoteId, AppError> {
            Ok(NoteId(1))
        }
        fn update_note(&self, _: NoteId, _: NoteDraft) -> Result<(), AppError> {
            Ok(())
        }
        fn move_note(&self, _: NoteId, _: ColumnId, _: usize) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_note(&self, _: NoteId) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[test]
    fn renders_the_columns_of_the_board() {
        let html = render_to_string(|| BoardApp(Rc::new(OneColumn)));
        assert!(html.contains("Kanban"));
        assert!(html.contains("Todo"));
    }
}
