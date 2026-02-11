use gtk4::gdk;
use gtk4::prelude::*;
use std::path::Path;

// ═══════════════════════════════════════════════
//  External Drag & Drop Source
// ═══════════════════════════════════════════════
//
// Enables dragging files AND folders OUT of Diptych
// to external targets: Desktop, other file managers,
// or web browser upload fields.
//
// The drag ghost is rendered via a cairo ImageSurface
// converted to GdkTexture — this always works because
// it doesn't depend on widget realization.

/// Attach an external drag source to any GTK4 widget.
///
/// Works for both files AND directories.
/// Shows a drag ghost with icon emoji + filename.
pub fn attach_file_drag_source(
    widget: &impl IsA<gtk4::Widget>,
    file_path: &Path,
    _icon_name: &str,
    is_dir: bool,
) {
    let drag_source = gtk4::DragSource::new();
    drag_source.set_actions(gdk::DragAction::COPY | gdk::DragAction::MOVE);

    let file_uri = path_to_file_uri(file_path);
    let path_owned = file_path.to_path_buf();

    // ── Prepare content ──
    {
        let file_uri = file_uri.clone();
        let path_owned = path_owned.clone();
        drag_source.connect_prepare(move |_source, _x, _y| {
            let g_file = gtk4::gio::File::for_path(&path_owned);
            let file_content = gdk::ContentProvider::for_value(&g_file.to_value());

            let uri_list = format!("{}\r\n", file_uri);
            let uri_content = gdk::ContentProvider::for_value(&uri_list.to_value());

            let union = gdk::ContentProvider::new_union(&[file_content, uri_content]);
            Some(union)
        });
    }

    // ── Ghost image via cairo → GdkTexture ──
    {
        let display_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        // Truncate long names for the ghost
        let short_name = if display_name.len() > 30 {
            format!("{}…", &display_name[..29])
        } else {
            display_name.clone()
        };

        let icon_emoji = if is_dir { "📁" } else { "📄" };
        let ghost_text = format!(" {} {} ", icon_emoji, short_name);

        drag_source.connect_drag_begin(move |source, _drag| {
            // Render the ghost label to a cairo surface → GdkTexture
            if let Some(texture) = render_ghost_texture(&ghost_text) {
                source.set_icon(Some(&texture), 16, 16);
            }
        });
    }

    widget.add_controller(drag_source);
}

/// Renders a text label onto a cairo ImageSurface and returns
/// it as a GdkTexture that can be used as a drag icon.
///
/// This approach bypasses widget realization issues entirely.
fn render_ghost_texture(text: &str) -> Option<gdk::Texture> {
    // Measure text with pango
    let font_map = pangocairo::FontMap::default();
    let context = font_map.create_context();
    let layout = pango::Layout::new(&context);
    layout.set_text(text);

    let font_desc = pango::FontDescription::from_string("Inter Semi-Bold 12");
    layout.set_font_description(Some(&font_desc));

    let (text_w, text_h) = layout.pixel_size();
    let pad_x = 12;
    let pad_y = 8;
    let w = text_w + pad_x * 2;
    let h = text_h + pad_y * 2;
    let radius = 10.0;

    // Create cairo surface
    let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h).ok()?;
    let cr = cairo::Context::new(&surface).ok()?;

    // Rounded rect background
    let (fw, fh) = (w as f64, h as f64);
    cr.new_sub_path();
    cr.arc(
        fw - radius,
        radius,
        radius,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    cr.arc(
        fw - radius,
        fh - radius,
        radius,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    cr.arc(
        radius,
        fh - radius,
        radius,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        radius,
        radius,
        radius,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();

    // Fill with dark bg
    cr.set_source_rgba(0.12, 0.12, 0.18, 0.92);
    let _ = cr.fill_preserve();

    // Border
    cr.set_source_rgba(0.45, 0.55, 0.75, 0.4);
    cr.set_line_width(1.0);
    let _ = cr.stroke();

    // Text
    cr.set_source_rgba(0.85, 0.87, 0.92, 1.0);
    cr.move_to(pad_x as f64, pad_y as f64);
    pangocairo::functions::show_layout(&cr, &layout);

    drop(cr);
    let _ = surface.flush();

    // Convert cairo surface → GdkTexture via GBytes
    let data = surface.data().ok()?;
    let bytes = glib::Bytes::from(&*data);
    let texture = gdk::MemoryTexture::new(
        w,
        h,
        gdk::MemoryFormat::B8g8r8a8Premultiplied,
        &bytes,
        (w * 4) as usize,
    );
    Some(texture.upcast())
}

/// Converts an absolute filesystem path to a `file:///` URI.
fn path_to_file_uri(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let encoded = abs
        .to_string_lossy()
        .replace(' ', "%20")
        .replace('#', "%23")
        .replace('?', "%3F");
    format!("file://{}", encoded)
}
