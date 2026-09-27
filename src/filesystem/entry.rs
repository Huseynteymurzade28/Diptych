use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ═══════════════════════════════════════════════
//  File / Directory Entry
// ═══════════════════════════════════════════════

/// Represents a single filesystem entry (file or directory).
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub extension: String,
    /// Lowercased name, precomputed: sorting compares it O(n log n) times.
    sort_key: String,
}

impl Entry {
    /// Builds an entry for `path` (follows symlinks for the metadata).
    pub fn from_path(path: &Path) -> Entry {
        let metadata = std::fs::metadata(path).ok();
        Entry::new(
            path.to_path_buf(),
            metadata.as_ref().is_some_and(|m| m.is_dir()),
            metadata.as_ref().map(|m| m.len()).unwrap_or(0),
            metadata.and_then(|m| m.modified().ok()),
        )
    }

    fn new(path: PathBuf, is_dir: bool, size: u64, modified: Option<SystemTime>) -> Entry {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        Entry {
            sort_key: name.to_lowercase(),
            name,
            extension: path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default(),
            path,
            is_dir,
            size,
            modified,
        }
    }

    /// Builds an entry from a GIO `FileInfo` enumerated inside `dir`
    /// (needs `standard::name,standard::type,standard::size,time::modified`).
    pub fn from_file_info(dir: &Path, info: &gio::FileInfo) -> Entry {
        let path = dir.join(info.name());
        let modified = info
            .has_attribute("time::modified")
            .then(|| info.attribute_uint64("time::modified"))
            .map(|secs| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs));
        Entry::new(
            path,
            info.file_type() == gio::FileType::Directory,
            info.size().max(0) as u64,
            modified,
        )
    }

    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }

    /// Display order: folders first, then case-insensitive by name.
    pub fn display_cmp(a: &Entry, b: &Entry) -> std::cmp::Ordering {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.sort_key.cmp(&b.sort_key))
    }

    /// Human-readable file size string.
    pub fn size_display(&self) -> String {
        if self.is_dir {
            return "—".to_string();
        }
        format_size(self.size)
    }

    /// Human-readable modified date.
    pub fn modified_display(&self) -> String {
        match self.modified {
            Some(time) => {
                let duration = time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default();
                let secs = duration.as_secs() as i64;
                let dt = chrono::DateTime::from_timestamp(secs, 0);
                match dt {
                    Some(d) => d.format("%Y-%m-%d %H:%M").to_string(),
                    None => "—".to_string(),
                }
            }
            None => "—".to_string(),
        }
    }
}

/// Human-readable byte count ("1.5 MB").
pub fn format_size(bytes: u64) -> String {
    let s = bytes as f64;
    if s < 1024.0 {
        format!("{} B", bytes)
    } else if s < 1024.0 * 1024.0 {
        format!("{:.1} KB", s / 1024.0)
    } else if s < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", s / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", s / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> Entry {
        Entry::new(PathBuf::from("/x").join(name), is_dir, 0, None)
    }

    #[test]
    fn folders_first_then_case_insensitive_names() {
        let mut entries = vec![
            entry("b.txt", false),
            entry("Zeta", true),
            entry("A.txt", false),
            entry("alpha", true),
        ];
        entries.sort_by(Entry::display_cmp);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["alpha", "Zeta", "A.txt", "b.txt"]);
    }

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    }
}
