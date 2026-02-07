use std::path::PathBuf;
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
