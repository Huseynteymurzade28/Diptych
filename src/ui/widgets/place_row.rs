use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation};

// ═══════════════════════════════════════════════
//  Sidebar Place Row & Go Up Row
// ═══════════════════════════════════════════════

/// Creates a sidebar navigation button.
pub fn create_place_row(label: &str, icon_name: &str) -> Button {
    let container = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();

    let icon = Image::builder().icon_name(icon_name).pixel_size(16).build();

    let lbl = Label::builder()
        .label(label)
        .xalign(0.0)
        .hexpand(true)
        .build();

    container.append(&icon);
    container.append(&lbl);

    Button::builder()
        .child(&container)
        .halign(Align::Fill)
        .has_frame(false)
        .css_classes(vec!["place-btn".to_string()])
        .build()
}

/// Creates the "Go Up" navigation button.
pub fn create_go_up_row() -> Button {
    let container = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();

    let icon = Image::builder()
        .icon_name("go-up-symbolic")
        .pixel_size(16)
        .build();

    let lbl = Label::builder()
        .label("Go Up")
        .xalign(0.0)
        .hexpand(true)
        .build();

    container.append(&icon);
    container.append(&lbl);

    Button::builder()
        .child(&container)
        .halign(Align::Fill)
        .has_frame(false)
        .css_classes(vec!["place-btn".to_string()])
        .build()
}
