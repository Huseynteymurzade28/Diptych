use crate::config::AppConfig;
use crate::filesystem::Entry;
use crate::ui::widgets::icon::icon_for_entry;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation};

// ═══════════════════════════════════════════════
//  List Row Widget
// ═══════════════════════════════════════════════

/// Creates a compact list-row widget.
pub fn create_file_row(entry: &Entry, config: &AppConfig) -> Button {
    let icon_name = icon_for_entry(entry);
    let icon_sz = (config.icon_size / 3).max(16).min(24);

    let container = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();

    let icon = Image::builder()
        .icon_name(icon_name)
        .pixel_size(icon_sz)
        .build();

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

    Button::builder()
        .child(&container)
        .halign(Align::Fill)
        .has_frame(false)
        .css_classes(vec!["file-row".to_string()])
        .build()
}
