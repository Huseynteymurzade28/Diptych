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
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();
                let is_dir = path.is_dir();

                if !include_hidden && file_name.starts_with('.') {
                    continue;
                }

                let metadata = fs::metadata(&path).ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata.and_then(|m| m.modified().ok());
                let extension = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default();

                file_list.push(Entry {
                    name: file_name,
                    path,
                    is_dir,
                    size,
                    modified,
                    extension,
                });
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
