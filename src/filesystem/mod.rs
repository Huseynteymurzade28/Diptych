use std::fs;
use std::path::{Path, PathBuf};

pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// Lists files in the given directory and returns them as a vector of Entry structs.
pub fn list_directory(path: &Path, include_hidden: bool) -> Vec<Entry> {
    let mut file_list = Vec::new();

    println!("Scanning directory: {:?}", path);
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = path.is_dir();

                    if !include_hidden && file_name.starts_with('.') {
                        continue;
                    }

                    file_list.push(Entry {
                        name: file_name,
                        path,
                        is_dir,
                    });
                }
            }
        }
        Err(e) => eprintln!("Failed to read directory entries: {}", e),
    }

    // Sort: Directories first, then files
    file_list.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    file_list
}

pub fn create_directory(parent: &Path, name: &str) -> std::io::Result<PathBuf> {
    let new_path = parent.join(name);
    fs::create_dir(&new_path)?;
    Ok(new_path)
}

pub fn create_file(parent: &Path, name: &str) -> std::io::Result<PathBuf> {
    let new_path = parent.join(name);
    fs::File::create(&new_path)?;
    Ok(new_path)
}
