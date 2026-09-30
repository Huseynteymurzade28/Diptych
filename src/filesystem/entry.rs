use crate::config::SortBy;
use chrono::Datelike;
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

    /// Order for the file views: folders first, then by `by` (ascending or
    /// descending), ties broken by name. Folders have no meaningful size,
    /// so they stay sorted by name when sorting by size.
    pub fn sort_cmp(a: &Entry, b: &Entry, by: SortBy, descending: bool) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        if a.is_dir != b.is_dir {
            return b.is_dir.cmp(&a.is_dir);
        }
        let by_name = || a.sort_key.cmp(&b.sort_key);
        let key = match by {
            SortBy::Name => Ordering::Equal,
            SortBy::Size if a.is_dir => Ordering::Equal,
            SortBy::Size => a.size.cmp(&b.size),
            SortBy::Modified => a.modified.cmp(&b.modified),
            SortBy::Type => a.extension.to_lowercase().cmp(&b.extension.to_lowercase()),
        };
        let order = key.then_with(by_name);
        if descending {
            order.reverse()
        } else {
            order
        }
    }

    /// Human-readable file size string.
    pub fn size_display(&self) -> String {
        if self.is_dir {
            return "—".to_string();
        }
        format_size(self.size)
    }

    /// Short, friendly modified date in local time: "Today, 14:05",
    /// "Yesterday, 09:12", "Sep 28", or "2025-03-01" for other years.
    pub fn modified_display(&self) -> String {
        match self.modified_local() {
            Some(dt) => friendly_date(dt, chrono::Local::now()),
            None => "—".to_string(),
        }
    }

    /// Full modified date and time in local time ("2026-09-30 21:17").
    pub fn modified_full(&self) -> String {
        match self.modified_local() {
            Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
            None => "—".to_string(),
        }
    }

    fn modified_local(&self) -> Option<chrono::DateTime<chrono::Local>> {
        self.modified.map(chrono::DateTime::<chrono::Local>::from)
    }
}

/// `when` relative to `now`, for file lists.
pub fn friendly_date<Tz: chrono::TimeZone>(
    when: chrono::DateTime<Tz>,
    now: chrono::DateTime<Tz>,
) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let (day, today) = (when.date_naive(), now.date_naive());
    if day == today {
        when.format("Today, %H:%M").to_string()
    } else if today.pred_opt() == Some(day) {
        when.format("Yesterday, %H:%M").to_string()
    } else if day.year_ce() == today.year_ce() && day < today {
        when.format("%b %-d").to_string()
    } else {
        when.format("%Y-%m-%d").to_string()
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
    fn sorts_by_key_with_folders_first() {
        let file = |name: &str, size: u64, age: u64| {
            let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000 - age);
            Entry::new(PathBuf::from("/x").join(name), false, size, Some(modified))
        };
        let mut entries = vec![
            file("b.txt", 10, 1),
            entry("Zeta", true),
            file("a.rs", 300, 5),
            entry("alpha", true),
            file("c.md", 20, 3),
        ];
        let names = |v: &[Entry]| v.iter().map(|e| e.name.clone()).collect::<Vec<_>>();
        let sort = |v: &mut Vec<Entry>, by, desc| v.sort_by(|a, b| Entry::sort_cmp(a, b, by, desc));

        sort(&mut entries, SortBy::Size, false);
        assert_eq!(names(&entries), ["alpha", "Zeta", "b.txt", "c.md", "a.rs"]);
        sort(&mut entries, SortBy::Size, true);
        // Descending flips the folders' order too, but they stay first.
        assert_eq!(names(&entries), ["Zeta", "alpha", "a.rs", "c.md", "b.txt"]);
        sort(&mut entries, SortBy::Modified, true);
        assert_eq!(names(&entries)[2..], ["b.txt", "c.md", "a.rs"]);
        sort(&mut entries, SortBy::Type, false);
        assert_eq!(names(&entries)[2..], ["c.md", "a.rs", "b.txt"]);
        sort(&mut entries, SortBy::Name, false);
        assert_eq!(names(&entries), ["alpha", "Zeta", "a.rs", "b.txt", "c.md"]);
    }

    #[test]
    fn friendly_dates() {
        use chrono::{TimeZone, Utc};
        let now = Utc.with_ymd_and_hms(2026, 9, 30, 18, 0, 0).unwrap();
        let at = |y, m, d, h, min| Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap();
        assert_eq!(friendly_date(at(2026, 9, 30, 9, 5), now), "Today, 09:05");
        assert_eq!(
            friendly_date(at(2026, 9, 29, 23, 59), now),
            "Yesterday, 23:59"
        );
        assert_eq!(friendly_date(at(2026, 9, 1, 12, 0), now), "Sep 1");
        assert_eq!(friendly_date(at(2025, 12, 31, 12, 0), now), "2025-12-31");
        // Clock skew: a date in the future gets the full form.
        assert_eq!(friendly_date(at(2026, 10, 2, 12, 0), now), "2026-10-02");
    }

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    }
}
