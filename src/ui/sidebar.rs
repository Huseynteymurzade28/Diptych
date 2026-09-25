use crate::ui::context_menu::name_submitter;
use crate::ui::state::AppState;
use crate::ui::widgets;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Label, Orientation, Popover, ScrolledWindow};
use std::path::{Path, PathBuf};
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Sidebar (Places), Path Bar and "New" popover
// ═══════════════════════════════════════════════

/// Builds the sidebar around `places` (filled later by `bind_places`).
pub fn build_sidebar(places: &Box) -> gtk4::Widget {
    let column = Box::builder().orientation(Orientation::Vertical).build();
    column.append(
        &Label::builder()
            .label("PLACES")
            .css_classes(["sidebar-title"])
            .halign(Align::Start)
            .margin_top(8)
            .build(),
    );
    column.append(places);

    ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&column)
        .css_classes(["sidebar"])
        .build()
        .upcast()
}

/// Standard places. Each row keeps its path as tooltip, which is also how
/// `refresh_places` finds the current one.
pub fn bind_places(state: &Rc<AppState>) {
    let places = [
        ("Home", "user-home-symbolic", dirs::home_dir()),
        ("Desktop", "user-desktop-symbolic", dirs::desktop_dir()),
        (
            "Documents",
            "folder-documents-symbolic",
            dirs::document_dir(),
        ),
        (
            "Downloads",
            "folder-download-symbolic",
            dirs::download_dir(),
        ),
        ("Pictures", "folder-pictures-symbolic", dirs::picture_dir()),
        ("Music", "folder-music-symbolic", dirs::audio_dir()),
        ("Videos", "folder-videos-symbolic", dirs::video_dir()),
        (
            "Computer",
            "drive-harddisk-symbolic",
            Some(PathBuf::from("/")),
        ),
    ];

    for (name, icon, path) in places {
        // XDG dirs that don't exist (or equal $HOME) would be duplicates.
        let Some(path) = path.filter(|p| p.is_dir()) else {
            continue;
        };
        if name != "Home" && Some(&path) == dirs::home_dir().as_ref() {
            continue;
        }
        let btn = widgets::create_place_row(name, icon);
        btn.set_tooltip_text(Some(&path.to_string_lossy()));
        let state_c = state.clone();
        btn.connect_clicked(move |_| state_c.navigate_to(path.clone()));
        state.places.append(&btn);
    }
}

/// Highlights the place matching the current folder.
pub fn refresh_places(state: &Rc<AppState>) {
    let current = state.current_path().to_string_lossy().to_string();
    let mut child = state.places.first_child();
    while let Some(row) = child {
        if row.tooltip_text().as_deref() == Some(current.as_str()) {
            row.add_css_class("active");
        } else {
            row.remove_css_class("active");
        }
        child = row.next_sibling();
    }
}

// ─── Path bar ───

/// Rebuilds the clickable path segments: `Home / projects / Diptych`.
pub fn refresh_path_bar(state: &Rc<AppState>) {
    let bar = &state.chrome.path_bar;
    while let Some(child) = bar.first_child() {
        bar.remove(&child);
    }

    let path = state.current_path();
    let segments = path_segments(&path, dirs::home_dir().as_deref());
    let last = segments.len().saturating_sub(1);
    for (i, (label, target)) in segments.into_iter().enumerate() {
        if i > 0 {
            bar.append(
                &Label::builder()
                    .label("/")
                    .css_classes(["path-sep"])
                    .build(),
            );
        }
        let btn = Button::builder()
            .label(&label)
            .has_frame(false)
            .css_classes(["path-segment"])
            .build();
        if i == last {
            btn.add_css_class("path-current");
        }
        let state_c = state.clone();
        btn.connect_clicked(move |_| state_c.navigate_to(target.clone()));
        bar.append(&btn);
    }

    // Keep the current folder in view when the path is longer than the bar.
    let bar = bar.clone();
    glib::idle_add_local_once(move || {
        if let Some(scroll) = bar
            .ancestor(ScrolledWindow::static_type())
            .and_downcast::<ScrolledWindow>()
        {
            let adj = scroll.hadjustment();
            adj.set_value(adj.upper());
        }
    });
}

/// `(label, path)` per segment. Paths under `home` start at "Home".
fn path_segments(path: &Path, home: Option<&Path>) -> Vec<(String, PathBuf)> {
    let (mut acc, rest, mut out) =
        match home.and_then(|h| path.strip_prefix(h).ok().map(|r| (h, r))) {
            Some((h, rel)) => (
                h.to_path_buf(),
                rel.to_path_buf(),
                vec![("Home".to_string(), h.to_path_buf())],
            ),
            None => (
                PathBuf::from("/"),
                path.strip_prefix("/").unwrap_or(path).to_path_buf(),
                vec![("/".to_string(), PathBuf::from("/"))],
            ),
        };
    for part in rest.components() {
        acc.push(part);
        out.push((part.as_os_str().to_string_lossy().to_string(), acc.clone()));
    }
    out
}

// ─── "New" popover (header bar) ───

pub fn setup_creation_popover(state: &Rc<AppState>) {
    let popover = Popover::builder().css_classes(["context-menu"]).build();

    let pop_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let title_label = Label::builder()
        .label("Create New")
        .css_classes(["context-menu-title"])
        .halign(Align::Start)
        .build();
    let entry = gtk4::Entry::builder().placeholder_text("Name…").build();

    let btn_row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(Align::End)
        .build();
    let create_file_btn = Button::builder()
        .label("File")
        .css_classes(["btn-secondary", "creation-btn"])
        .build();
    let create_folder_btn = Button::builder()
        .label("Folder")
        .css_classes(["btn-primary", "creation-btn"])
        .build();
    btn_row.append(&create_file_btn);
    btn_row.append(&create_folder_btn);

    pop_box.append(&title_label);
    pop_box.append(&entry);
    pop_box.append(&btn_row);
    popover.set_child(Some(&pop_box));
    state.chrome.new_button.set_popover(Some(&popover));

    let submit_folder = name_submitter(&entry, &popover, {
        let state = state.clone();
        move |name| state.create(name, true)
    });
    let submit_file = name_submitter(&entry, &popover, {
        let state = state.clone();
        move |name| state.create(name, false)
    });

    // Enter creates a folder (the primary button).
    {
        let submit = submit_folder.clone();
        entry.connect_activate(move |_| submit());
    }
    create_folder_btn.connect_clicked(move |_| submit_folder());
    create_file_btn.connect_clicked(move |_| submit_file());
}

#[cfg(test)]
mod tests {
    use super::path_segments;
    use std::path::{Path, PathBuf};

    fn labels(path: &str, home: Option<&str>) -> Vec<String> {
        path_segments(Path::new(path), home.map(Path::new))
            .into_iter()
            .map(|(l, _)| l)
            .collect()
    }

    #[test]
    fn segments_under_home_start_at_home() {
        assert_eq!(
            labels("/home/u/dev/Diptych", Some("/home/u")),
            ["Home", "dev", "Diptych"]
        );
        assert_eq!(labels("/home/u", Some("/home/u")), ["Home"]);
        let segs = path_segments(Path::new("/home/u/dev"), Some(Path::new("/home/u")));
        assert_eq!(segs[1].1, PathBuf::from("/home/u/dev"));
    }

    #[test]
    fn segments_outside_home_start_at_root() {
        assert_eq!(labels("/usr/share", Some("/home/u")), ["/", "usr", "share"]);
        assert_eq!(labels("/", Some("/home/u")), ["/"]);
        // A sibling that merely shares the prefix is not "under home".
        assert_eq!(
            labels("/home/user2", Some("/home/u")),
            ["/", "home", "user2"]
        );
    }
}
