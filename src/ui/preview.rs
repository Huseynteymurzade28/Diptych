use crate::thumbnail;
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::*;
use gtk4::{Align, Box, Image, Label, Orientation, Picture, Spinner};
use std::path::Path;

// ═══════════════════════════════════════════════
//  Media Preview System (Smart Previews)
// ═══════════════════════════════════════════════
//
// Generates thumbnails for images and video keyframes.
// Loading happens asynchronously so the main UI thread never freezes.

/// File types that support preview thumbnails.
fn is_image(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico"
    )
}

fn is_video(ext: &str) -> bool {
    matches!(ext, "mp4" | "mkv" | "avi" | "mov" | "webm")
}

/// Returns true if the file at `path` supports a media preview.
pub fn supports_preview(path: &Path) -> bool {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    is_image(&ext) || is_video(&ext)
}

// ═══════════════════════════════════════════════
//  Preview Widget Builder
// ═══════════════════════════════════════════════

/// Builds a preview widget for the given file path.
/// Returns a container that shows a spinner while loading,
/// then replaces it with the actual thumbnail.
#[allow(dead_code)]
pub fn build_preview_widget(file_path: &Path, max_width: i32, max_height: i32) -> Box {
    let container = Box::builder()
        .orientation(Orientation::Vertical)
        .halign(Align::Center)
        .valign(Align::Center)
        .spacing(8)
        .css_classes(vec!["preview-container".to_string()])
        .build();

    let ext = file_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if is_image(&ext) {
        build_image_preview(&container, file_path, max_width, max_height);
    } else if is_video(&ext) {
        build_video_placeholder(&container, file_path);
    }

    container
}

// ─── Image Preview ───

/// Loads an image thumbnail asynchronously using GLib idle_add.
fn build_image_preview(container: &Box, file_path: &Path, max_w: i32, max_h: i32) {
    // Show spinner while loading
    let spinner = Spinner::builder()
        .spinning(true)
        .width_request(48)
        .height_request(48)
        .halign(Align::Center)
        .build();
    container.append(&spinner);

    let loading_label = Label::builder()
        .label("Loading preview…")
        .css_classes(vec!["preview-loading-label".to_string()])
        .build();
    container.append(&loading_label);

    let path = file_path.to_path_buf();
    let container_weak = container.downgrade();

    // Async load: run the heavy pixbuf decode off the next idle tick
    glib::idle_add_local_once(move || {
        let Some(container) = container_weak.upgrade() else {
            return;
        };

        // Try to load and scale the image
        match load_scaled_pixbuf(&path, max_w, max_h) {
            Some(pixbuf) => {
                // Remove spinner + label
                while let Some(child) = container.first_child() {
                    container.remove(&child);
                }

                let picture = Picture::for_pixbuf(&pixbuf);
                picture.set_can_shrink(true);
                picture.set_halign(Align::Center);
                picture.set_valign(Align::Center);
                picture.add_css_class("preview-image");
                container.append(&picture);

                // Show dimensions
                let w = pixbuf.width();
                let h = pixbuf.height();
                let dim_label = Label::builder()
                    .label(&format!("{}×{}", w, h))
                    .css_classes(vec!["preview-dimension-label".to_string()])
                    .halign(Align::Center)
                    .build();
                container.append(&dim_label);
            }
            None => {
                // Remove spinner
                while let Some(child) = container.first_child() {
                    container.remove(&child);
                }

                let err_label = Label::builder()
                    .label("⚠ Could not load preview")
                    .css_classes(vec!["preview-error-label".to_string()])
                    .halign(Align::Center)
                    .build();
                container.append(&err_label);
            }
        }
    });
}

/// Loads a pixbuf at the given path, scaling it to fit within max dimensions.
fn load_scaled_pixbuf(path: &Path, max_w: i32, max_h: i32) -> Option<Pixbuf> {
    // First try loading at constrained size for performance
    match Pixbuf::from_file_at_scale(path, max_w, max_h, true) {
        Ok(pb) => Some(pb),
        Err(_) => {
            // Fallback: try loading full image then scaling
            Pixbuf::from_file(path)
                .ok()
                .map(|pb| {
                    let (ow, oh) = (pb.width() as f64, pb.height() as f64);
                    let scale = (max_w as f64 / ow).min(max_h as f64 / oh).min(1.0);
                    let new_w = (ow * scale).max(1.0) as i32;
                    let new_h = (oh * scale).max(1.0) as i32;
                    pb.scale_simple(new_w, new_h, gtk4::gdk_pixbuf::InterpType::Bilinear)
                })
                .flatten()
        }
    }
}

// ─── Video Preview (FFmpeg Thumbnail) ───

/// Extracts a video keyframe via the thumbnail cache/generator system
/// and shows it as a preview. Falls back to a play-icon placeholder
/// if FFmpeg is unavailable or the file is corrupted.
fn build_video_placeholder(container: &Box, file_path: &Path) {
    let overlay_box = Box::builder()
        .orientation(Orientation::Vertical)
        .halign(Align::Center)
        .valign(Align::Center)
        .spacing(8)
        .css_classes(vec!["preview-video-placeholder".to_string()])
        .build();

    // Try to load a cached thumbnail first, otherwise generate one
    let cache = thumbnail::ThumbnailCache::new();
    let thumb_available = if let Some(cached_path) = cache.get(file_path) {
        load_scaled_pixbuf(&cached_path, 320, 240)
    } else {
        let dest = cache.thumb_path(file_path);
        if thumbnail::generate_video_thumbnail(file_path, &dest, 320, 240) {
            load_scaled_pixbuf(&dest, 320, 240)
        } else {
            None
        }
    };

    if let Some(pixbuf) = thumb_available {
        let picture = Picture::for_pixbuf(&pixbuf);
        picture.set_can_shrink(true);
        picture.set_halign(Align::Center);
        picture.set_valign(Align::Center);
        picture.add_css_class("preview-image");
        overlay_box.append(&picture);
    } else {
        // Fallback: play icon
        let icon = Image::builder()
            .icon_name("media-playback-start-symbolic")
            .pixel_size(64)
            .halign(Align::Center)
            .css_classes(vec!["preview-play-icon".to_string()])
            .build();
        overlay_box.append(&icon);
    }

    let name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Video".to_string());

    let name_label = Label::builder()
        .label(&name)
        .css_classes(vec!["preview-video-name".to_string()])
        .halign(Align::Center)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .max_width_chars(24)
        .build();

    let hint_label = Label::builder()
        .label("Click to play externally")
        .css_classes(vec!["preview-hint-label".to_string()])
        .halign(Align::Center)
        .build();

    overlay_box.append(&name_label);
    overlay_box.append(&hint_label);
    container.append(&overlay_box);
}

// ═══════════════════════════════════════════════
//  Hover Tooltip Preview (Compact)
// ═══════════════════════════════════════════════

/// Builds a small thumbnail suitable for tooltip / hover preview (96×96).
/// Uses the disk cache so repeated hovers are instant.
pub fn build_tooltip_preview(file_path: &Path) -> Option<Image> {
    let ext = file_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if !is_image(&ext) && !is_video(&ext) {
        return None;
    }

    // Try cache first
    let cache = thumbnail::ThumbnailCache::new();
    if let Some(cached) = cache.get(file_path) {
        return load_scaled_pixbuf(&cached, 96, 96).map(|pb| {
            let img = Image::from_pixbuf(Some(&pb));
            img.add_css_class("preview-tooltip-image");
            img
        });
    }

    // For images we can generate synchronously (fast enough for tooltip)
    if is_image(&ext) {
        load_scaled_pixbuf(file_path, 96, 96).map(|pb| {
            let img = Image::from_pixbuf(Some(&pb));
            img.add_css_class("preview-tooltip-image");
            img
        })
    } else {
        // Video — don't block for FFmpeg on tooltip, show nothing
        None
    }
}
