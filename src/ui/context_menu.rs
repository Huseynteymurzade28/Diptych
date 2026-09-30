use crate::ui::state::AppState;
use gtk4::prelude::*;
use gtk4::{
    Align, Button, Entry as GtkEntry, GestureClick, Label, Orientation, Popover, Separator, Widget,
};
use std::path::Path;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Right-Click Context Menu System
// ═══════════════════════════════════════════════
//
// Background context menu — right-click on empty space
//   → "New Folder", "New File", "Open Terminal Here", your actions.toml
//     actions, "Select All", "Refresh"
// (The item menu is a shared `gio::Menu` in `file_view.rs`.)
//
// All actual work is delegated to `AppState`.

// ═══════════════════════════════════════════════
//  Background Context Menu (empty area)
// ═══════════════════════════════════════════════

/// Attaches a right-click context menu to the content area background.
pub fn attach_background_context_menu(target: &impl IsA<Widget>, state: &Rc<AppState>) {
    let target = target.as_ref().clone();
    let popover = new_menu_popover(&target);

    let new_folder_btn = context_menu_button("folder-new-symbolic", "New Folder");
    let new_file_btn = context_menu_button("document-new-symbolic", "New File");
    let terminal_btn = context_menu_button("utilities-terminal-symbolic", "Open Terminal Here");
    terminal_btn.set_action_name(Some("win.open-terminal"));
    let bookmark_btn = context_menu_button("starred-symbolic", "Bookmark This Folder");
    bookmark_btn.set_action_name(Some("win.bookmark"));
    let select_all_btn = context_menu_button("edit-select-all-symbolic", "Select All");
    select_all_btn.set_action_name(Some("win.select-all"));
    let refresh_btn = context_menu_button("view-refresh-symbolic", "Refresh");
    refresh_btn.set_action_name(Some("win.refresh"));

    // actions.toml entries for the folder itself; rebuilt on every popup.
    let custom_box = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .build();

    let menu_box = menu_box();
    menu_box.append(&new_folder_btn);
    menu_box.append(&new_file_btn);
    menu_box.append(&terminal_btn);
    menu_box.append(&bookmark_btn);
    menu_box.append(&custom_box);
    menu_box.append(&menu_separator());
    menu_box.append(&select_all_btn);
    menu_box.append(&refresh_btn);
    popover.set_child(Some(&menu_box));

    for (btn, is_dir) in [(&new_folder_btn, true), (&new_file_btn, false)] {
        let popover_c = popover.clone();
        let target_c = target.clone();
        let state = state.clone();
        btn.connect_clicked(move |_| {
            popover_c.popdown();
            let title = if is_dir {
                "Create Folder"
            } else {
                "Create File"
            };
            let state = state.clone();
            show_name_dialog(&target_c, title, "Create", "", move |name| {
                state.create(name, is_dir)
            });
        });
    }
    for btn in [&terminal_btn, &bookmark_btn, &select_all_btn, &refresh_btn] {
        let popover_c = popover.clone();
        btn.connect_clicked(move |_| popover_c.popdown());
    }

    let gesture = GestureClick::builder().button(3).build();
    let state = state.clone();
    gesture.connect_pressed(move |_gesture, _n, x, y| {
        // The menu is about the folder, so actions run on the folder.
        state.clear_selection();
        while let Some(child) = custom_box.first_child() {
            custom_box.remove(&child);
        }
        let entries = state.applicable_actions();
        if !entries.is_empty() {
            custom_box.append(&menu_separator());
        }
        for (index, action) in entries {
            let icon = action.icon.as_deref().unwrap_or("system-run-symbolic");
            let btn = context_menu_button(icon, &action.name);
            btn.set_action_name(Some("win.run-action"));
            btn.set_action_target_value(Some(&(index as i32).to_variant()));
            let popover_c = popover.clone();
            btn.connect_clicked(move |_| popover_c.popdown());
            custom_box.append(&btn);
        }
        // Position the popover at the click coordinates
        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
    });
    target.add_controller(gesture);
}

// ═══════════════════════════════════════════════
//  Name Dialog (create / rename)
// ═══════════════════════════════════════════════

/// Asks for a new name for `path`, anchored to `anchor`.
pub fn show_rename_dialog(state: &Rc<AppState>, anchor: &Widget, path: &Path) {
    let old_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let state = state.clone();
    let path = path.to_path_buf();
    show_name_dialog(anchor, "Rename", "Rename", &old_name, move |name| {
        state.rename(&path, name)
    });
}

/// Shows a small popover anchored to `anchor` asking for a name.
/// `submit` performs the operation; its error is shown on the entry.
pub fn show_name_dialog(
    anchor: &Widget,
    title: &str,
    button_label: &str,
    initial: &str,
    submit: impl Fn(&str) -> Result<(), String> + 'static,
) {
    let dialog = Popover::builder()
        .css_classes(vec!["context-menu".to_string()])
        .build();
    dialog.set_parent(anchor);
    // One-shot popover: detach it once closed so it doesn't pile up.
    dialog.connect_closed(|d| {
        let d = d.clone();
        glib::idle_add_local_once(move || d.unparent());
    });

    let vbox = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let label = Label::builder()
        .label(title)
        .css_classes(vec!["context-menu-title".to_string()])
        .halign(Align::Start)
        .build();

    let entry = GtkEntry::builder()
        .placeholder_text("Name…")
        .text(initial)
        .build();

    let button = Button::builder()
        .label(button_label)
        .css_classes(vec!["btn-primary".to_string()])
        .build();

    vbox.append(&label);
    vbox.append(&entry);
    vbox.append(&button);
    dialog.set_child(Some(&vbox));

    let run = name_submitter(&entry, &dialog, submit);
    {
        let run = run.clone();
        entry.connect_activate(move |_| run());
    }
    button.connect_clicked(move |_| run());

    dialog.popup();

    // Pre-select the name without its extension ("photo|.jpg").
    let stem_len = Path::new(initial)
        .file_stem()
        .map(|s| s.to_string_lossy().chars().count())
        .unwrap_or(0);
    entry.grab_focus();
    entry.select_region(0, stem_len as i32);
}

/// Returns a closure that passes `entry`'s text to `submit`.
/// On success the entry is cleared and `popover` closed; on failure the
/// entry is marked invalid and the reason is shown as its tooltip.
pub fn name_submitter(
    entry: &GtkEntry,
    popover: &Popover,
    submit: impl Fn(&str) -> Result<(), String> + 'static,
) -> Rc<dyn Fn()> {
    entry.connect_changed(|e| {
        e.remove_css_class("error");
        e.set_tooltip_text(None);
    });

    let entry = entry.clone();
    let popover = popover.clone();
    Rc::new(move || match submit(entry.text().as_str()) {
        Ok(()) => {
            entry.set_text("");
            popover.popdown();
        }
        Err(msg) => {
            eprintln!("{}", msg);
            entry.add_css_class("error");
            entry.set_tooltip_text(Some(&msg));
        }
    })
}

// ═══════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════

fn new_menu_popover(parent: &Widget) -> Popover {
    let popover = Popover::builder()
        .css_classes(vec!["context-menu".to_string()])
        .has_arrow(true)
        .halign(Align::Start)
        .build();
    popover.set_parent(parent);

    // Ensure popover is unparented when the target widget is destroyed
    let popover_destroy = popover.clone();
    parent.connect_destroy(move |_| popover_destroy.unparent());
    popover
}

fn menu_box() -> gtk4::Box {
    gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(4)
        .margin_end(4)
        .build()
}

fn menu_separator() -> Separator {
    Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(4)
        .margin_bottom(4)
        .build()
}

/// Creates a styled context menu button with icon + label.
fn context_menu_button(icon_name: &str, label_text: &str) -> Button {
    let hbox = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();

    let icon = gtk4::Image::builder()
        .icon_name(icon_name)
        .pixel_size(16)
        .build();

    let label = Label::builder()
        .label(label_text)
        .xalign(0.0)
        .hexpand(true)
        .build();

    hbox.append(&icon);
    hbox.append(&label);

    Button::builder()
        .child(&hbox)
        .has_frame(false)
        .css_classes(vec!["context-menu-item".to_string()])
        .build()
}
