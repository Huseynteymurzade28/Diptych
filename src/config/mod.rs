// ─── Config Module ───
// Manages all user-configurable settings and their disk persistence.

pub mod actions;
pub mod edit;
pub mod keybindings;
pub mod layout;
pub mod persistence;
pub mod types;
pub mod watch;

// Re-export most commonly used items for convenience.
pub use actions::Actions;
pub use keybindings::Keybindings;
pub use layout::LayoutConfig;
pub use types::{AppConfig, GroupBy, IconTheme, OpenWith, SortBy, ViewMode};
