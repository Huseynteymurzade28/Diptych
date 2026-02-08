use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use glib::object::ObjectExt;
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::*;
use gtk4::Image;

use super::cache::ThumbnailCache;
use super::generator;

// ═══════════════════════════════════════════════
//  Async Thumbnail Worker
// ═══════════════════════════════════════════════
//
// Integrates with the GTK4 main loop:
//
//   1. Check the disk cache — if a fresh thumbnail exists, load it
//      synchronously (fast path, just a PNG read).
//   2. Otherwise, spawn the heavy work (decode + resize / FFmpeg)
//      on a background **std thread**, and once finished, schedule
//      a UI update on the main thread with `glib::idle_add_local_once`.
//
// The caller receives an `Image` widget immediately. It starts as
// a placeholder icon and swaps to the thumbnail once ready.
//
// GTK4 widgets are NOT Send/Sync, so we use `glib::SendWeakRef`
// (a thread-safe wrapper around `WeakRef`) to pass the widget handle
// safely to the background thread's idle callback.

/// Global, lazily-initialised cache handle (thread-safe).
fn cache() -> &'static ThumbnailCache {
    static INSTANCE: OnceLock<ThumbnailCache> = OnceLock::new();
    INSTANCE.get_or_init(ThumbnailCache::new)
}

/// Request a thumbnail for `source_path`.
///
/// Returns an `Image` widget that will initially show a placeholder icon.
/// When the thumbnail is ready (cache hit or freshly generated) it will
/// be swapped in automatically on the GTK main thread.
///
/// `icon_size` controls the pixel size of the placeholder while waiting.
pub fn request_thumbnail(source_path: &Path, icon_size: i32) -> Image {
    let ext = source_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Determine the right placeholder icon
    let placeholder_icon = if super::is_thumbable_video(&ext) {
        "video-x-generic-symbolic"
    } else {
        "image-x-generic-symbolic"
    };

    let image = Image::builder()
        .icon_name(placeholder_icon)
        .pixel_size(icon_size)
        .halign(gtk4::Align::Center)
        .build();
    image.add_css_class("thumbnail-placeholder");

    // ── Fast path: cache hit ──
    if let Some(cached) = cache().get(source_path) {
        if let Some(pb) = load_pixbuf_scaled(&cached, icon_size) {
            image.set_from_pixbuf(Some(&pb));
            image.remove_css_class("thumbnail-placeholder");
            image.add_css_class("thumbnail-loaded");
            return image;
        }
    }

    // ── Slow path: generate in background ──
    let source = source_path.to_path_buf();
    let thumb_dest = cache().thumb_path(source_path);
    let pixel_size = icon_size;

    // `SendWeakRef` is a Send+Sync wrapper around glib::WeakRef.
    // It can be safely moved into a std::thread closure.
    let send_weak: glib::SendWeakRef<Image> = image.downgrade().into();

    std::thread::spawn(move || {
        let ok = generator::generate_thumbnail(&source, &thumb_dest);

        // Schedule UI update on the main GTK thread.
        // `MainContext::default().invoke()` is the thread-safe way to
        // dispatch a closure to the GLib main loop from any thread.
        glib::MainContext::default().invoke(move || {
            let Some(image) = send_weak.upgrade() else {
                return; // widget was dropped
            };

            if ok {
                if let Some(pb) = load_pixbuf_scaled(&thumb_dest, pixel_size) {
                    image.set_from_pixbuf(Some(&pb));
                    image.remove_css_class("thumbnail-placeholder");
                    image.add_css_class("thumbnail-loaded");
                    return;
                }
            }

            // Generation failed — show error icon so the user knows
            image.set_icon_name(Some("dialog-warning-symbolic"));
            image.remove_css_class("thumbnail-placeholder");
            image.add_css_class("thumbnail-error");
        });
    });

    image
}

// ─── Helpers ───

/// Loads a PNG thumbnail and scales it to fit `size × size`.
fn load_pixbuf_scaled(path: &PathBuf, size: i32) -> Option<Pixbuf> {
    Pixbuf::from_file_at_scale(path, size, size, true).ok()
}
