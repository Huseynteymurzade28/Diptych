// ─── Filesystem Module ───
// File entry types, directory operations, and grouping logic.

mod entry;
mod grouping;
mod ops;

pub use entry::{format_size, Entry};
pub use ops::{
    count_entries, create_directory, create_file, delete_permanently, list_directory, move_to_trash,
};
