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
}

impl Entry {
    /// Builds an entry for `path` (follows symlinks for the metadata).
    pub fn from_path(path: &Path) -> Entry {
        let metadata = std::fs::metadata(path).ok();
        Entry {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string()),
            path: path.to_path_buf(),
            is_dir: metadata.as_ref().is_some_and(|m| m.is_dir()),
            size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
            modified: metadata.and_then(|m| m.modified().ok()),
            extension: path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default(),
        }
    }

    /// Human-readable file size string.
    pub fn size_display(&self) -> String {
        if self.is_dir {
            return "—".to_string();
        }
        let s = self.size as f64;
        if s < 1024.0 {
            format!("{} B", self.size)
        } else if s < 1024.0 * 1024.0 {
            format!("{:.1} KB", s / 1024.0)
        } else if s < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", s / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", s / (1024.0 * 1024.0 * 1024.0))
        }
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
