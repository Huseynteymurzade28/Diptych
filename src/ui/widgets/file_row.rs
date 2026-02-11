use crate::config::{AppConfig, IconTheme};
use crate::filesystem::Entry;
use crate::thumbnail;
use crate::ui::drag_source;
use crate::ui::widgets::icon::{icon_css_class, icon_for_entry_themed};
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation};

// ═══════════════════════════════════════════════
//  List Row Widget
// ═══════════════════════════════════════════════

/// Creates a compact list-row widget.
pub fn create_file_row(entry: &Entry, config: &AppConfig) -> Button {
    let icon_name = icon_for_entry_themed(entry, &config.icon_theme);
    let icon_sz = (config.icon_size / 3).max(16).min(24);

    let container = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();

    // Check if this file supports a thumbnail preview
    let ext = entry.extension.to_lowercase();
    let has_thumb = !entry.is_dir && thumbnail::supports_thumbnail(&ext);

    let icon: Image = if has_thumb {
        thumbnail::request_thumbnail(&entry.path, icon_sz)
    } else {
        let icon_classes = if config.icon_theme == IconTheme::Colorful {
            vec![icon_css_class(entry).to_string()]
        } else {
            vec![]
        };

        Image::builder()
            .icon_name(icon_name)
            .pixel_size(icon_sz)
            .css_classes(icon_classes)
            .build()
    };

    let name_label = Label::builder()
        .label(&entry.name)
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .build();

    container.append(&icon);
    container.append(&name_label);

    // Optional metadata columns
    if config.show_file_size {
        let size_label = Label::builder()
            .label(&entry.size_display())
            .css_classes(vec!["file-row-meta".to_string()])
            .halign(Align::End)
            .width_chars(8)
            .xalign(1.0)
            .build();
        container.append(&size_label);
    }
    if config.show_modified_date {
        let date_label = Label::builder()
            .label(&entry.modified_display())
            .css_classes(vec!["file-row-meta".to_string()])
            .halign(Align::End)
            .width_chars(16)
            .xalign(1.0)
            .build();
        container.append(&date_label);
    }

    let btn = Button::builder()
        .child(&container)
        .halign(Align::Fill)
        .has_frame(false)
        .css_classes(vec!["file-row".to_string()])
        .build();

    // ── External drag & drop source (files AND folders) ──
    drag_source::attach_file_drag_source(&btn, &entry.path, icon_name, entry.is_dir);

    btn
}
