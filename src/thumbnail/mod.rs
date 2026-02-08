// ─── Thumbnail Module ───
// Async thumbnail generation with disk caching for images and videos.
//
// Architecture:
//   cache.rs   — Disk cache under ~/.cache/diptych/thumbnails
//   generator.rs — Image resizing (via `image` crate) & video frame capture (FFmpeg)
//   worker.rs  — Async task spawner: non-blocking generation with lazy loading

pub mod cache;
pub mod generator;
pub mod worker;

pub use cache::ThumbnailCache;
#[allow(unused_imports)]
pub use generator::{generate_image_thumbnail, generate_thumbnail, generate_video_thumbnail};
pub use worker::request_thumbnail;

/// Default thumbnail dimensions (pixels).
pub const THUMB_WIDTH: u32 = 192;
pub const THUMB_HEIGHT: u32 = 192;

/// File extensions that support thumbnail generation.
pub fn supports_thumbnail(ext: &str) -> bool {
    is_thumbable_image(ext) || is_thumbable_video(ext)
}

pub fn is_thumbable_image(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico")
}

pub fn is_thumbable_video(ext: &str) -> bool {
    matches!(ext, "mp4" | "mkv" | "avi" | "mov" | "webm")
}
