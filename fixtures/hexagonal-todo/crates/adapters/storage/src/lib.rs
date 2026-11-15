//! Local storage driven adapter: JSON board snapshot in browser localStorage (wasm32) or a file (native).

mod backend;
mod records;
mod repository;

pub use backend::Backend;
pub use repository::LocalStorageBoardRepository;
