use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation};

/// Creates a standard file list row button with a system icon.
pub fn create_file_row(name: &str, is_dir: bool) -> Button {
    let icon_name = if is_dir { "folder" } else { "text-x-generic" };
    
    let container = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();

    let icon = Image::builder()
        .icon_name(icon_name)
        .pixel_size(20)
        .build();

    let label = Label::builder()
        .label(name)
        .xalign(0.0) // Align text to the left
        .hexpand(true) // Take available horizontal space
        .build();

    container.append(&icon);
    container.append(&label);

    Button::builder()
        .child(&container)
        .halign(Align::Fill)
        .has_frame(false)
        .css_classes(vec!["file-row".to_string()]) // Helpful for future theming
        .build()
}
