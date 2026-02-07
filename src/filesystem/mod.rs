// ─── Filesystem Module ───
// File entry types, directory operations, and grouping logic.

mod entry;
mod grouping;
mod ops;

pub use entry::Entry;
pub use grouping::group_entries;
pub use ops::{create_directory, create_file, list_directory};
