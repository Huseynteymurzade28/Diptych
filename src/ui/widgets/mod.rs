// ─── Widgets Submodule ───
// Reusable GTK widget factories for file cards, rows, and place buttons.

pub mod file_card;
pub mod file_row;
pub mod icon;
pub mod place_row;

// Re-export the most-used factory functions at module level.
pub use file_card::create_file_card;
pub use file_row::create_file_row;
pub use place_row::{create_go_up_row, create_place_row};
