use crate::config::layout::InspectorField;
use crate::filesystem::{self, Entry};
use crate::ui::state::AppState;
use crate::ui::{context_menu, preview};
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

    let (entry, is_selection) = match state.selected() {
        Some(path) => (Entry::from_path(&path), true),
        None => (Entry::from_path(&state.current_path()), false),
    };
    let fields = state.layout.borrow().inspector.fields.clone();
    let content_type = content_type(&entry);

    pane.append(&preview_area(&entry, &content_type));

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
                .ellipsize(gtk4::pango::EllipsizeMode::Middle)
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
        pane.append(&actions(state, &entry));
    } else {
        let hint = Label::builder()
            .label(match state.config.borrow().open_with {
                crate::config::OpenWith::DoubleClick => {
                    "Click an item to see its details. Double-click to open it."
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

fn preview_area(entry: &Entry, content_type: &str) -> gtk4::Widget {
    if !entry.is_dir && preview::supports_preview(&entry.path) {
        return preview::build_preview_widget(&entry.path, 260, 200).upcast();
    }
    // Symbolic icon with GIO's own fallback chain: never a "missing" icon.
    let icon = gio::content_type_get_symbolic_icon(content_type);
    let image = Image::builder()
        .gicon(&icon)
        .pixel_size(96)
        .css_classes([crate::ui::widgets::icon::icon_css_class(entry)])
        .build();
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
            let n =
                filesystem::list_directory(&entry.path, state.config.borrow().show_hidden).len();
            Some((
                "Contains",
                format!("{} item{}", n, if n == 1 { "" } else { "s" }),
            ))
        }
        InspectorField::Size => Some(("Size", entry.size_display())),
        InspectorField::Modified => Some(("Modified", entry.modified_display())),
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

fn actions(state: &Rc<AppState>, entry: &Entry) -> Box {
    let column = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();

    let open = Button::builder()
        .label(if entry.is_dir { "Open Folder" } else { "Open" })
        .css_classes(["btn-primary"])
        .build();
    {
        let (state, entry) = (state.clone(), entry.clone());
        open.connect_clicked(move |_| state.activate(&entry));
    }

    let row = Box::builder().spacing(8).homogeneous(true).build();
    let rename = Button::builder()
        .label("Rename")
        .css_classes(["btn-secondary"])
        .build();
    {
        let (state, path) = (state.clone(), entry.path.clone());
        rename.connect_clicked(move |btn| {
            let old = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let (state, path) = (state.clone(), path.clone());
            context_menu::show_name_dialog(
                btn.upcast_ref(),
                "Rename",
                "Rename",
                &old,
                move |name| state.rename(&path, name),
            );
        });
    }
    let trash = Button::builder()
        .label("Move to Trash")
        .css_classes(["btn-secondary", "context-menu-danger"])
        .build();
    {
        let (state, path) = (state.clone(), entry.path.clone());
        trash.connect_clicked(move |_| state.trash(&path));
    }
    row.append(&rename);
    row.append(&trash);

    column.append(&open);
    column.append(&row);
    column
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
