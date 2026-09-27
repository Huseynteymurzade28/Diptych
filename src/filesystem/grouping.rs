use std::time::SystemTime;

use crate::config::GroupBy;
use crate::filesystem::Entry;

// ═══════════════════════════════════════════════
//  Grouping Logic
// ═══════════════════════════════════════════════

impl Entry {
    /// Returns a category label used for grouping.
    pub fn group_key(&self, group_by: &GroupBy) -> String {
        match group_by {
            GroupBy::None => String::new(),
            GroupBy::Type => {
                if self.is_dir {
                    "📁 Folders".to_string()
                } else if self.extension.is_empty() {
                    "📄 Other".to_string()
                } else {
                    format!("📄 .{}", self.extension.to_uppercase())
                }
            }
            GroupBy::Date => self
                .modified
                .map(|t| {
                    let dur = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
                    let secs = dur.as_secs() as i64;
                    chrono::DateTime::from_timestamp(secs, 0)
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                })
                .unwrap_or_else(|| "Unknown".to_string()),
            GroupBy::Name => {
                let first = self.name.chars().next().unwrap_or('#');
                if first.is_alphabetic() {
                    first.to_uppercase().to_string()
                } else {
                    "#".to_string()
                }
            }
        }
    }
}
