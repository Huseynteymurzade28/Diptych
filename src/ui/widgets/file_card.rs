use crate::config::{AppConfig, IconTheme};
use crate::filesystem::Entry;
use crate::thumbnail;
use crate::ui::widgets::icon::{icon_css_class, icon_for_entry_themed};
use gtk4::prelude::*;
use gtk4::{Align, Box, Image, Label, Orientation};

// ═══════════════════════════════════════════════
//  Grid Card Widget
// ═══════════════════════════════════════════════

/// Creates a card-style widget for grid view (a `GridView` item).
pub fn create_file_card(entry: &Entry, config: &AppConfig) -> Box {
    let icon_name = icon_for_entry_themed(entry, &config.icon_theme);

    let card_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();

    // Check if this file supports a thumbnail preview
    let ext = entry.extension.to_lowercase();
    let has_thumb = !entry.is_dir && thumbnail::supports_thumbnail(&ext);

    let icon: Image = if has_thumb {
        // Async thumbnail — shows placeholder first, swaps in the real image
        thumbnail::request_thumbnail(&entry.path, config.icon_size)
    } else {
        // Folders always wear the folder color; files are tinted by kind
        // with the Colorful icon theme only.
        let icon_classes = if entry.is_dir || config.icon_theme == IconTheme::Colorful {
            vec![icon_css_class(entry).to_string()]
        } else {
            vec![]
        };

        Image::builder()
            .icon_name(icon_name)
            .pixel_size(config.icon_size)
            .halign(Align::Center)
            .css_classes(icon_classes)
            .build()
    };

    // Two lines at most, middle-ellipsized: long names never widen the card
    // (the grid's column width follows its widest card).
    let name_label = Label::builder()
        .label(&entry.name)
        .css_classes(vec!["file-card-name".to_string()])
        .halign(Align::Center)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .lines(2)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .width_chars(1)
        .max_width_chars(1)
        .justify(gtk4::Justification::Center)
        .build();

    card_box.append(&icon);
    card_box.append(&name_label);

    // One quiet line: the size for files. Dates live in the list view and
    // the inspector, so the grid stays calm.
    if config.show_file_size && !entry.is_dir {
        let size_label = Label::builder()
            .label(entry.size_display())
            .css_classes(vec!["file-card-meta".to_string()])
            .halign(Align::Center)
            .build();
        card_box.append(&size_label);
    }

    // Card size adapts to icon_size
    let card_width = config.icon_size.max(48) + 64;
    card_box.add_css_class("file-card");
    card_box.set_halign(Align::Center);
    card_box.set_width_request(card_width);
    card_box
}
