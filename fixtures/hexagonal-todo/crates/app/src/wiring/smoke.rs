//! Native smoke run: builds a small board through the service and prints it.

use todo_application::ports::inbound::{AppError, BoardService, ColumnDraft, NoteDraft};

fn column(title: &str, is_close: bool) -> ColumnDraft {
    ColumnDraft {
        title: title.to_string(),
        color: "#336699".to_string(),
        is_close,
    }
}

/// Creates Todo and Done (closing) columns, moves one note into Done and prints the board.
pub fn run(service: &dyn BoardService) -> Result<(), AppError> {
    let todo = service.create_column(column("Todo", false))?;
    let done = service.create_column(column("Done", true))?;
    let note = service.create_note(
        todo,
        NoteDraft {
            title: "Write docs".to_string(),
            body: "Explain the hexagon".to_string(),
        },
    )?;
    service.move_note(note, done, 0)?;
    for column in service.board()?.columns {
        println!("{} (is_close = {})", column.title, column.is_close);
        for note in column.notes {
            println!("  - {} (closed = {})", note.title, note.closed);
        }
    }
    Ok(())
}
