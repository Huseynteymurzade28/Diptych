use gtk4::prelude::*;
use gtk4::{Align, Box, Label, Orientation};

// ═══════════════════════════════════════════════
//  Inspector Bar
// ═══════════════════════════════════════════════

/// Builds the bottom inspector bar (selected file info + open button).
/// Returns (bar_widget, info_label, open_button).
pub fn build_inspector_bar() -> (Box, Label) {
    let inspector_bar = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(6)
        .margin_bottom(8)
        .margin_start(16)
        .margin_end(16)
        .build();

    let inspector_info = Label::builder()
        .label("Select a file to inspect")
        .css_classes(vec!["inspector-subtitle".to_string()])
        .halign(Align::Start)
        .hexpand(true)
        .xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .build();

    inspector_bar.append(&inspector_info);

    (inspector_bar, inspector_info)
}
