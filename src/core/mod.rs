// ─── Core Module ───
// Framework-agnostic domain logic: theme definitions, color palettes.

pub mod text;
pub mod theme;

pub use text::truncate_chars;
pub use theme::Theme;
