use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use image::imageops::FilterType;
use image::ImageReader;

use super::{THUMB_HEIGHT, THUMB_WIDTH};

// ═══════════════════════════════════════════════
//  FFmpeg Availability Check
// ═══════════════════════════════════════════════

/// Cached result of whether `ffmpeg` is reachable on $PATH.
/// Checked once per process lifetime — avoids spawning a failing
/// process for every single video file in a large directory.
static FFMPEG_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Probes for `ffmpeg` by running `ffmpeg -version`.
fn check_ffmpeg() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Returns `true` if FFmpeg is installed and runnable.
pub fn is_ffmpeg_available() -> bool {
    *FFMPEG_AVAILABLE.get_or_init(|| {
        let available = check_ffmpeg();
        if !available {
            eprintln!(
                "[thumb-gen] FFmpeg bulunamadı — video önizlemeleri devre dışı. \
                 Etkinleştirmek için FFmpeg yükleyin: https://ffmpeg.org"
            );
        }
        available
    })
}

// ═══════════════════════════════════════════════
//  Thumbnail Generator
// ═══════════════════════════════════════════════
//
// • Images  — decoded with the `image` crate, resized with a fast
//             Lanczos3 filter, and saved as PNG.
// • Videos  — a single frame is extracted at t=1s using FFmpeg
//             (`ffmpeg -ss 1 -i <video> -frames:v 1 …`).

// ─── Image Thumbnails ───

/// Generates a thumbnail for an image file and writes it to `out_path`.
/// Returns `true` on success.
pub fn generate_image_thumbnail(source: &Path, out_path: &Path, width: u32, height: u32) -> bool {
    let reader = match ImageReader::open(source) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "[thumb-gen] Cannot open image {}: {}",
                source.display(),
                e
            );
            return false;
        }
    };

    // Guess format so we can decode exotic types too
    let reader = match reader.with_guessed_format() {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "[thumb-gen] Cannot guess format for {}: {}",
                source.display(),
                e
            );
            return false;
        }
    };

    let img = match reader.decode() {
        Ok(img) => img,
        Err(e) => {
            eprintln!(
                "[thumb-gen] Failed to decode {}: {}",
                source.display(),
                e
            );
            return false;
        }
    };

    // Resize preserving aspect ratio — Lanczos3 for quality, fast enough for thumbnails
    let thumb = img.resize(width, height, FilterType::Lanczos3);

    match thumb.save(out_path) {
        Ok(()) => true,
        Err(e) => {
            eprintln!(
                "[thumb-gen] Failed to save thumbnail to {}: {}",
                out_path.display(),
                e
            );
            false
        }
    }
}

// ─── Video Thumbnails (FFmpeg) ───

/// Extracts a single frame from a video at ~1 second and saves it to `out_path`.
/// Requires `ffmpeg` to be available on `$PATH`.
/// Returns `true` on success, `false` if FFmpeg is missing or extraction fails.
pub fn generate_video_thumbnail(source: &Path, out_path: &Path, width: u32, height: u32) -> bool {
    // Early exit if FFmpeg is not installed — no point spawning a doomed process
    if !is_ffmpeg_available() {
        return false;
    }

    // Build the scale filter string: scale to fit within width×height, keep aspect ratio
    let scale_filter = format!(
        "scale='min({w},iw)':min'({h},ih)':force_original_aspect_ratio=decrease",
        w = width,
        h = height,
    );

    let status = Command::new("ffmpeg")
        .args([
            "-y",                         // overwrite output
            "-ss", "1",                   // seek to 1 second
            "-i",
        ])
        .arg(source)                      // input file (may contain spaces)
        .args([
            "-frames:v", "1",            // grab one frame
            "-vf", &scale_filter,
            "-q:v", "2",                 // high quality JPEG → we save as PNG below
        ])
        .arg(out_path)                    // output path (PNG by extension)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!(
                "[thumb-gen] FFmpeg exited with {} for {}",
                s,
                source.display()
            );
            false
        }
        Err(e) => {
            // FFmpeg probably not installed
            eprintln!(
                "[thumb-gen] Failed to run FFmpeg for {}: {}",
                source.display(),
                e
            );
            false
        }
    }
}

// ─── Convenience ───

/// Auto-dispatches to the correct generator based on extension.
pub fn generate_thumbnail(source: &Path, out_path: &Path) -> bool {
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if super::is_thumbable_image(&ext) {
        generate_image_thumbnail(source, out_path, THUMB_WIDTH, THUMB_HEIGHT)
    } else if super::is_thumbable_video(&ext) {
        generate_video_thumbnail(source, out_path, THUMB_WIDTH, THUMB_HEIGHT)
    } else {
        false
    }
}
