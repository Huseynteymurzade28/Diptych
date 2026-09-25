use gio::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

use crate::filesystem::Entry;

// ═══════════════════════════════════════════════
//  Directory Operations
// ═══════════════════════════════════════════════

/// Lists files in the given directory and returns them as a vector of `Entry`.
pub fn list_directory(path: &Path, include_hidden: bool) -> Vec<Entry> {
    let mut file_list = Vec::new();

    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if !include_hidden && entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                file_list.push(Entry::from_path(&entry.path()));
            }
        }
        Err(e) => eprintln!("Failed to read directory entries: {}", e),
    }

    // Sort: directories first, then files alphabetically
    file_list.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    file_list
}

/// Creates a new directory inside `parent`.
pub fn create_directory(parent: &Path, name: &str) -> std::io::Result<PathBuf> {
    let new_path = parent.join(name);
    fs::create_dir(&new_path)?;
    Ok(new_path)
}

/// Creates a new empty file inside `parent`.
pub fn create_file(parent: &Path, name: &str) -> std::io::Result<PathBuf> {
    let new_path = parent.join(name);
    fs::File::create(&new_path)?;
    Ok(new_path)
}

/// Moves `path` to the freedesktop trash so it can be restored later.
pub fn move_to_trash(path: &Path) -> Result<(), glib::Error> {
    gio::File::for_path(path).trash(gio::Cancellable::NONE)
}

/// Permanently deletes `path` (recursively for directories). Cannot be undone.
pub fn delete_permanently(path: &Path) -> std::io::Result<()> {
    // symlink_metadata: a symlink to a directory must be unlinked, not followed.
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
