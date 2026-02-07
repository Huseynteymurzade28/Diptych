use crate::config::IconTheme;
use crate::filesystem::Entry;

// ═══════════════════════════════════════════════
//  Dynamic Icon System
// ═══════════════════════════════════════════════
//
// Supports multiple switchable icon themes:
//   - Minimal   : clean symbolic icons (GTK standard)
//   - Colorful  : category-colored semantic icons
//   - Outline   : thin outline-style symbolic icons

/// Maps a filesystem entry to the appropriate icon name based on the active icon theme.
#[allow(dead_code)]
pub fn icon_for_entry(entry: &Entry) -> &'static str {
    icon_for_entry_themed(entry, &IconTheme::Minimal)
}

/// Maps a filesystem entry to an icon name using the specified icon theme.
pub fn icon_for_entry_themed(entry: &Entry, theme: &IconTheme) -> &'static str {
    if entry.is_dir {
        return dir_icon(theme);
    }
    match theme {
        IconTheme::Minimal => minimal_icon(&entry.extension),
        IconTheme::Colorful => colorful_icon(&entry.extension),
        IconTheme::Outline => outline_icon(&entry.extension),
    }
}

// ─── Directory Icons ───

fn dir_icon(theme: &IconTheme) -> &'static str {
    match theme {
        IconTheme::Minimal => "folder-symbolic",
        IconTheme::Colorful => "folder",
        IconTheme::Outline => "folder-open-symbolic",
    }
}

// ─── Minimal Theme ───
// Clean, uniform symbolic icons — all files use the same base icon per category.
// Designed for minimal visual clutter.

fn minimal_icon(ext: &str) -> &'static str {
    match ext {
        // All source code → same single icon
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "rb" | "swift" | "kt"
        | "cs" | "lua" | "sh" | "fish" | "zsh" | "bash" | "html" | "htm" | "css" => {
            "text-x-generic-symbolic"
        }
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
        // Everything else (docs, text, config)
        _ => "text-x-generic-symbolic",
    }
}

// ─── Colorful Theme ───
// Vivid, category-specific icons — more visual distinction.

fn colorful_icon(ext: &str) -> &'static str {
    match ext {
        // Rust
        "rs" => "application-x-executable",
        // Python
        "py" => "text-x-python",
        // JavaScript / TypeScript
        "js" | "ts" => "text-x-script",
        // C / C++
        "c" | "cpp" | "h" => "text-x-csrc",
        // Java
        "java" => "text-x-java",
        // Go
        "go" => "text-x-generic",
        // Shell scripts
        "sh" | "fish" | "zsh" | "bash" => "application-x-shellscript",
        // Other source code
        "rb" | "swift" | "kt" | "cs" | "lua" => "text-x-script",
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" => "image-x-generic",
        // Audio
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" => "audio-x-generic",
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "video-x-generic",
        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "package-x-generic",
        // PDF
        "pdf" => "x-office-document",
        // Web
        "html" | "htm" | "css" => "text-html",
        // Data / Config
        "json" | "toml" | "yaml" | "yml" | "xml" => "text-x-generic",
        // Markdown
        "md" => "text-x-generic",
        // Plain text
        "txt" | "log" | "csv" => "text-x-generic",
        _ => "text-x-generic",
    }
}

// ─── Outline Theme ───
// Detailed symbolic icons — each major file type gets its own distinctive icon.
// More visual variety than Minimal, but still monochrome symbolic style.

fn outline_icon(ext: &str) -> &'static str {
    match ext {
        // Source code — uses script icon to distinguish from plain text
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "rb" | "swift" | "kt"
        | "cs" | "lua" => "text-x-script-symbolic",
        // Shell scripts — distinct executable icon
        "sh" | "fish" | "zsh" | "bash" => "application-x-executable-symbolic",
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" => "image-x-generic-symbolic",
        // Vector images
        "svg" => "image-x-generic-symbolic",
        // Audio
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" => "audio-x-generic-symbolic",
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "video-x-generic-symbolic",
        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "package-x-generic-symbolic",
        // PDF / Documents
        "pdf" => "x-office-document-symbolic",
        // Web
        "html" | "htm" => "text-html-symbolic",
        "css" => "text-x-preview-symbolic",
        // Markdown
        "md" => "x-office-document-symbolic",
        // Config files
        "json" | "toml" | "yaml" | "yml" | "xml" => "emblem-system-symbolic",
        // Plain text / logs
        "txt" | "log" | "csv" => "accessories-text-editor-symbolic",
        _ => "text-x-generic-symbolic",
    }
}

// ═══════════════════════════════════════════════
//  Icon Badge / CSS Class Helpers
// ═══════════════════════════════════════════════

/// Returns a CSS class name for coloring the icon based on file type.
/// Used by the Colorful theme to tint icons by category.
pub fn icon_css_class(entry: &Entry) -> &'static str {
    if entry.is_dir {
        return "icon-folder";
    }
    match entry.extension.as_str() {
        "rs" => "icon-rust",
        "py" => "icon-python",
        "js" | "ts" => "icon-js",
        "c" | "cpp" | "h" => "icon-c",
        "java" | "kt" => "icon-java",
        "go" => "icon-go",
        "sh" | "fish" | "zsh" | "bash" | "lua" | "rb" | "swift" | "cs" => "icon-script",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" => "icon-image",
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" => "icon-audio",
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "icon-video",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "icon-archive",
        "pdf" => "icon-pdf",
        "html" | "htm" | "css" => "icon-web",
        "md" | "txt" | "log" | "csv" => "icon-text",
        "json" | "toml" | "yaml" | "yml" | "xml" => "icon-config",
        _ => "icon-default",
    }
}
