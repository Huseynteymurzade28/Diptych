use crate::config::AppConfig;
use crate::filesystem::Entry;
use crate::ui::widgets::icon::icon_for_entry;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation};

// ═══════════════════════════════════════════════
//  Grid Card Widget
// ═══════════════════════════════════════════════

/// Creates a card-style widget for grid view.
pub fn create_file_card(entry: &Entry, config: &AppConfig) -> Button {
    let icon_name = icon_for_entry(entry);

    let card_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    let icon = Image::builder()
        .icon_name(icon_name)
        .pixel_size(config.icon_size)
        .halign(Align::Center)
        .build();

    let name_label = Label::builder()
        .label(&truncate_name(&entry.name, 18))
        .css_classes(vec!["file-card-name".to_string()])
        .halign(Align::Center)
        .wrap(true)
        .max_width_chars(16)
        .justify(gtk4::Justification::Center)
        .build();
    name_label.set_tooltip_text(Some(&entry.name));

    card_box.append(&icon);
    card_box.append(&name_label);

    // Metadata lines
    if config.show_file_size && !entry.is_dir {
        let size_label = Label::builder()
            .label(&entry.size_display())
            .css_classes(vec!["file-card-meta".to_string()])
            .halign(Align::Center)
            .build();
        card_box.append(&size_label);
    }
    if config.show_modified_date {
        let date_label = Label::builder()
            .label(&entry.modified_display())
            .css_classes(vec!["file-card-meta".to_string()])
            .halign(Align::Center)
            .build();
        card_box.append(&date_label);
    }

    // Card size adapts to icon_size
    let card_width = (config.icon_size as i32).max(48) + 40;
    let btn = Button::builder()
        .child(&card_box)
        .css_classes(vec!["file-card".to_string()])
        .width_request(card_width)
        .has_frame(false)
        .build();

    btn
}

// ─── Helpers ───

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}…", &name[..max_len - 1])
    }
}
