use crate::config::layout::InspectorField;
use crate::filesystem::{self, Entry};
use crate::ui::preview;
use crate::ui::state::AppState;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation};
use std::path::Path;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Inspector Pane — the second half of the diptych
// ═══════════════════════════════════════════════
//
// Shows the selected item (preview, details, actions), or a summary of
// the current folder when nothing is selected. Which details appear is
// set by `[inspector] fields` in layout.toml.

/// The pane's root; `refresh` fills it.
pub fn build_pane() -> Box {
    Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .css_classes(["inspector-pane"])
        .build()
}

pub fn refresh(state: &Rc<AppState>) {
    let pane = &state.inspector;
    while let Some(child) = pane.first_child() {
        pane.remove(&child);
    }

    let selection = state.selection();
    if selection.len() > 1 {
        multi_selection(pane, &selection);
        return;
    }

    let (entry, is_selection) = match state.selected() {
        Some(path) => (Entry::from_path(&path), true),
        None => (Entry::from_path(&state.current_path()), false),
    };
    let fields = state.layout.borrow().inspector.fields.clone();
    let content_type = content_type(&entry);

    let system_icons = state.config.borrow().icon_theme == crate::config::IconTheme::System;
    pane.append(&preview_area(&entry, &content_type, system_icons));

    let title = Label::builder()
        .label(display_name(&entry))
        .css_classes(["inspector-title"])
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .selectable(true)
        .xalign(0.0)
        .build();
    pane.append(&title);
    if fields.contains(&InspectorField::Kind) {
        let kind = Label::builder()
            .label(gio::content_type_get_description(&content_type).as_str())
            .css_classes(["inspector-subtitle"])
            .xalign(0.0)
            .wrap(true)
            .build();
        pane.append(&kind);
    }

    let rows = gtk4::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .css_classes(["inspector-details"])
        .build();
    let mut row = 0;
    for field in &fields {
        if let Some((label, value)) = field_value(*field, &entry, state) {
            let key = Label::builder()
                .label(label)
                .css_classes(["inspector-meta-label"])
                .xalign(0.0)
                .valign(Align::Start)
                .build();
            let val = Label::builder()
                .label(&value)
                .tooltip_text(&value)
                .css_classes(["inspector-meta-value"])
                .xalign(0.0)
                .hexpand(true)
                .selectable(true)
                // Wrap rather than ellipsize: in a narrow pane an ellipsized
                // value shrinks to just "…" (#24).
                .wrap(true)
                .wrap_mode(gtk4::pango::WrapMode::WordChar)
                .width_chars(8)
                .build();
            rows.attach(&key, 0, row, 1, 1);
            rows.attach(&val, 1, row, 1, 1);
            row += 1;
        }
    }
    if row > 0 {
        pane.append(&rows);
    }

    if is_selection {
        pane.append(&actions());
    } else {
        let hint = Label::builder()
            .label(match state.config.borrow().open_with {
                crate::config::OpenWith::DoubleClick => {
                    "Select an item to see its details here. Double-click to open it."
                }
                crate::config::OpenWith::SingleClick => {
                    "Right-click an item for its actions. Click to open it."
                }
            })
            .css_classes(["inspector-subtitle"])
            .wrap(true)
            .xalign(0.0)
            .build();
        pane.append(&hint);
    }
}

// ─── Pieces ───

/// Summary for several selected items: count, total size, bulk actions.
fn multi_selection(pane: &Box, paths: &[std::path::PathBuf]) {
    let entries: Vec<Entry> = paths.iter().map(|p| Entry::from_path(p)).collect();
    let folders = entries.iter().filter(|e| e.is_dir).count();
    let files = entries.len() - folders;

    let icon = Image::builder()
        .icon_name("edit-select-all-symbolic")
        .pixel_size(96)
        .hexpand(true)
        .valign(Align::Center)
        .build();
    let frame = Box::builder()
        .height_request(160)
        .css_classes(["inspector-preview"])
        .build();
    frame.append(&icon);
    pane.append(&frame);

    pane.append(
        &Label::builder()
            .label(format!("{} items selected", entries.len()))
            .css_classes(["inspector-title"])
            .xalign(0.0)
            .build(),
    );
    let plural = |n: usize, word: &str| format!("{} {}{}", n, word, if n == 1 { "" } else { "s" });
    let mut kinds = vec![];
    if folders > 0 {
        kinds.push(plural(folders, "folder"));
    }
    if files > 0 {
        let bytes: u64 = entries.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
        kinds.push(format!(
            "{} ({})",
            plural(files, "file"),
            filesystem::format_size(bytes)
        ));
    }
    pane.append(
        &Label::builder()
            .label(kinds.join(", "))
            .css_classes(["inspector-subtitle"])
            .xalign(0.0)
            .wrap(true)
            .build(),
    );

    let row = Box::builder().spacing(8).homogeneous(true).build();
    let trash = action_button("user-trash-symbolic", "Trash", "win.trash-selection");
    trash.set_tooltip_text(Some("Move to Trash"));
    row.append(&trash);
    row.append(&action_button(
        "edit-delete-symbolic",
        "Delete…",
        "win.delete-selection",
    ));
    pane.append(&row);
}

/// A quiet pill button with an icon and a label, bound to a `win.*` action.
fn action_button(icon: &str, label: &str, action: &str) -> Button {
    let content = adw::ButtonContent::builder()
        .icon_name(icon)
        .label(label)
        .build();
    Button::builder()
        .child(&content)
        .action_name(action)
        .css_classes(["inspector-action"])
        .build()
}

fn display_name(entry: &Entry) -> String {
    if entry.path == Path::new("/") {
        "/".into()
    } else if Some(entry.path.as_path()) == dirs::home_dir().as_deref() {
        "Home".into()
    } else {
        entry.name.clone()
    }
}

/// MIME type via GIO, so kinds and icons match the rest of the desktop.
fn content_type(entry: &Entry) -> String {
    if entry.is_dir {
        return "inode/directory".into();
    }
    gio::content_type_guess(Some(&entry.path), None)
        .0
        .to_string()
}

fn preview_area(entry: &Entry, content_type: &str, system_icons: bool) -> gtk4::Widget {
    if !entry.is_dir && preview::supports_preview(&entry.path) {
        return preview::build_preview_widget(&entry.path, 260, 200).upcast();
    }
    // GIO's own fallback chains: never a "missing" icon. The System style
    // shows the icon theme's full-color icon, the others a tinted symbolic.
    let image = if system_icons {
        Image::builder()
            .gicon(&crate::ui::widgets::icon::system_icon(entry))
            .pixel_size(96)
            .build()
    } else {
        Image::builder()
            .gicon(&gio::content_type_get_symbolic_icon(content_type))
            .pixel_size(96)
            .css_classes([crate::ui::widgets::icon::icon_css_class(entry)])
            .build()
    };
    let frame = Box::builder()
        .height_request(160)
        .halign(Align::Fill)
        .css_classes(["inspector-preview"])
        .build();
    image.set_hexpand(true);
    image.set_valign(Align::Center);
    frame.append(&image);
    frame.upcast()
}

fn field_value(
    field: InspectorField,
    entry: &Entry,
    state: &AppState,
) -> Option<(&'static str, String)> {
    let meta = std::fs::metadata(&entry.path).ok();
    match field {
        InspectorField::Kind => None, // shown as the subtitle
        InspectorField::Size if entry.is_dir => {
            let n = filesystem::count_entries(&entry.path, state.config.borrow().show_hidden);
            Some((
                "Contains",
                format!("{} item{}", n, if n == 1 { "" } else { "s" }),
            ))
        }
        InspectorField::Size => Some(("Size", entry.size_display())),
        InspectorField::Modified => Some(("Modified", entry.modified_full())),
        InspectorField::Created => {
            let created = meta?.created().ok()?;
            let dt: chrono::DateTime<chrono::Local> = created.into();
            Some(("Created", dt.format("%Y-%m-%d %H:%M").to_string()))
        }
        InspectorField::Dimensions => {
            if entry.is_dir {
                return None;
            }
            let (_, w, h) = gtk4::gdk_pixbuf::Pixbuf::file_info(&entry.path)?;
            Some(("Dimensions", format!("{} × {}", w, h)))
        }
        InspectorField::Location => {
            let parent = entry.path.parent()?;
            Some(("Location", abbreviate_home(parent)))
        }
        InspectorField::Permissions => {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta?.permissions().mode();
            Some(("Permissions", format!("{} ({:o})", rwx(mode), mode & 0o777)))
        }
    }
}

/// Rename and trash for the selected item. Opening is a double-click (or
/// Enter) away, so it gets no button here.
fn actions() -> Box {
    let row = Box::builder().spacing(8).homogeneous(true).build();
    row.append(&action_button(
        "document-edit-symbolic",
        "Rename",
        "win.rename-selection",
    ));
    let trash = action_button("user-trash-symbolic", "Trash", "win.trash-selection");
    trash.set_tooltip_text(Some("Move to Trash"));
    trash.add_css_class("danger");
    row.append(&trash);
    row
}

// ─── Formatting helpers ───

pub fn abbreviate_home(path: &Path) -> String {
    match dirs::home_dir().and_then(|home| path.strip_prefix(&home).ok().map(Path::to_path_buf)) {
        Some(rel) if rel.as_os_str().is_empty() => "~".into(),
        Some(rel) => format!("~/{}", rel.display()),
        None => path.display().to_string(),
    }
}

fn rwx(mode: u32) -> String {
    let bit = |mask: u32, c: char| if mode & mask != 0 { c } else { '-' };
    [
        bit(0o400, 'r'),
        bit(0o200, 'w'),
        bit(0o100, 'x'),
        bit(0o040, 'r'),
        bit(0o020, 'w'),
        bit(0o010, 'x'),
        bit(0o004, 'r'),
        bit(0o002, 'w'),
        bit(0o001, 'x'),
    ]
    .iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::rwx;

    #[test]
    fn permission_strings() {
        assert_eq!(rwx(0o755), "rwxr-xr-x");
        assert_eq!(rwx(0o644), "rw-r--r--");
        assert_eq!(rwx(0o100600), "rw-------");
    }
}
