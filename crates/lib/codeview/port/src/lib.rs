//! The code viewer's contract (docs/proposals/code-viewer.md): what a host provides (data types
//! and the [`Provider`] trait), the HTTP endpoints the viewer calls, and the iframe messages
//! between the host page and the viewer. No dependency on any host application.

mod endpoint;
mod message;
mod model;
#[cfg(test)]
mod tests;

pub use endpoint::*;
pub use message::{ToHost, ToViewer};
pub use model::*;

/// What a host application implements (server side) to feed the viewer. `path` is always a
/// relative path without `.` or `..` components (the host router rejects anything else).
pub trait Provider: Send + Sync + 'static {
    fn views(&self) -> Vec<View>;
    fn file(&self, path: &str) -> Option<FileText>;
    fn marks(&self, path: &str) -> Vec<Mark>;
    fn bands(&self, path: &str) -> Vec<Band>;
    fn definition(&self, path: &str, pos: Pos) -> Vec<Location>;
    fn references(&self, path: &str, pos: Pos) -> Vec<Location>;
}
