use crate::filesystem::Entry;

// ═══════════════════════════════════════════════
//  Icon Mapping
// ═══════════════════════════════════════════════

/// Maps a filesystem entry to the appropriate GTK symbolic icon name.
pub fn icon_for_entry(entry: &Entry) -> &'static str {
    if entry.is_dir {
        return "folder-symbolic";
    }
    match entry.extension.as_str() {
        // Source code
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "rb" | "swift" | "kt"
        | "cs" | "lua" | "sh" | "fish" | "zsh" | "bash" => "text-x-script-symbolic",

        // Images
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" => {
            "image-x-generic-symbolic"
        }

        // Audio
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" => "audio-x-generic-symbolic",

        // Video
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "video-x-generic-symbolic",

        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "package-x-generic-symbolic",

        // Documents
        "pdf" => "x-office-document-symbolic",

        // Text / Config
        "md" | "txt" | "log" | "csv" | "json" | "toml" | "yaml" | "yml" | "xml" => {
            "text-x-generic-symbolic"
        }

        _ => "text-x-generic-symbolic",
    }
}
