pub mod shape;
pub mod util;

// A re-export: callers may reach `shape::unit` as `mx_base::unit`.
pub use shape::unit;
