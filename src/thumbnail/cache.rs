use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ═══════════════════════════════════════════════
//  Thumbnail Disk Cache
// ═══════════════════════════════════════════════
//
// Thumbnails are stored under:
//   ~/.cache/diptych/thumbnails/<sha256_hex>.png
//
// The hash key is derived from the absolute file path + last-modified
// timestamp, so a cache entry is automatically invalidated when the
// source file changes.

/// Manages the on-disk thumbnail cache directory.
#[derive(Debug, Clone)]
pub struct ThumbnailCache {
    cache_dir: PathBuf,
}

impl ThumbnailCache {
    /// Creates (if necessary) and returns a new cache handle.
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("diptych")
            .join("thumbnails");

        if !cache_dir.exists() {
            if let Err(e) = fs::create_dir_all(&cache_dir) {
                eprintln!("[thumbnail-cache] Failed to create cache dir: {}", e);
            }
        }

        Self { cache_dir }
    }

    /// Returns the cached thumbnail path if it exists **and** is still fresh
    /// (i.e. the source file hasn't been modified since the thumbnail was written).
    pub fn get(&self, source: &Path) -> Option<PathBuf> {
        let thumb_path = self.thumb_path(source);
        if !thumb_path.exists() {
            return None;
        }

        // Freshness check: compare source mtime with cache mtime
        let src_mtime = fs::metadata(source)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let cache_mtime = fs::metadata(&thumb_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        if src_mtime > cache_mtime {
            // Source is newer — invalidate
            let _ = fs::remove_file(&thumb_path);
            return None;
        }

        Some(thumb_path)
    }

    /// Returns the path where a thumbnail *should* be stored (may not exist yet).
    pub fn thumb_path(&self, source: &Path) -> PathBuf {
        let key = self.cache_key(source);
        self.cache_dir.join(format!("{}.png", key))
    }

    /// Produces a deterministic hex key for the given source file.
    /// Incorporates the canonical path + last-modified timestamp so that
    /// edits to the file automatically bust the cache.
    fn cache_key(&self, source: &Path) -> String {
        let canonical = source
            .canonicalize()
            .unwrap_or_else(|_| source.to_path_buf());
        let mtime = fs::metadata(source)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut hasher = Sha256::new();
        hasher.update(canonical.to_string_lossy().as_bytes());
        hasher.update(mtime.to_le_bytes());
        hex::encode(hasher.finalize())
    }

    /// Total number of cached thumbnails (for diagnostics / settings UI).
    #[allow(dead_code)]
    pub fn entry_count(&self) -> usize {
        fs::read_dir(&self.cache_dir)
            .map(|rd| rd.count())
            .unwrap_or(0)
    }

    /// Deletes all cached thumbnails.
    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(rd) = fs::read_dir(&self.cache_dir) {
            for entry in rd.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}
