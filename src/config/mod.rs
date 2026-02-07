// ─── Config Module ───
// Manages all user-configurable settings and their disk persistence.

pub mod persistence;
pub mod types;

// Re-export most commonly used items for convenience.
pub use types::{AppConfig, GroupBy, IconTheme, ViewMode};
