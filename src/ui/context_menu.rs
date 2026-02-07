use crate::config::AppConfig;
use crate::filesystem;
use crate::ui::content::refresh_content;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, Entry as GtkEntry, GestureClick, Label, Orientation, Popover, Separator,
    Widget,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Right-Click Context Menu System
// ═══════════════════════════════════════════════
//
// Two context menus:
//   1. Background context menu — right-click on empty space
//      → "New Folder", "New File"
//   2. File/item context menu  — right-click on a file entry
//      → "Open", "Rename", "Delete"

// ═══════════════════════════════════════════════
//  Background Context Menu (empty area)
// ═══════════════════════════════════════════════

/// Attaches a right-click context menu to the content area background.
/// Provides "New Folder" and "New File" options.
pub fn attach_background_context_menu(
    target: &impl IsA<Widget>,
    current_path: Rc<RefCell<PathBuf>>,
    content_box: Box,
    inspector_info: Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
    let popover = build_background_popover(
        current_path,
        content_box,
        inspector_info,
        selected_file_path,
        config,
    );

    popover.set_parent(target.as_ref());
    popover.set_has_arrow(true);
    popover.set_halign(Align::Start);

    // Ensure popover is unparented when the target widget is destroyed
    let popover_destroy = popover.clone();
    target.as_ref().connect_destroy(move |_| {
        popover_destroy.unparent();
    });

    let gesture = GestureClick::builder()
        .button(3) // right mouse button
        .build();

    let popover_c = popover.clone();
    gesture.connect_pressed(move |_gesture, _n, x, y| {
        // Position the popover at the click coordinates
        popover_c.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover_c.popup();
    });

    target.as_ref().add_controller(gesture);
}

fn build_background_popover(
    current_path: Rc<RefCell<PathBuf>>,
    content_box: Box,
    inspector_info: Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) -> Popover {
    let popover = Popover::builder()
        .css_classes(vec!["context-menu".to_string()])
        .build();

    let menu_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(4)
        .margin_end(4)
        .build();

    // ── "New Folder" button ──
    let new_folder_btn = context_menu_button("folder-new-symbolic", "New Folder");
    // ── "New File" button ──
    let new_file_btn = context_menu_button("document-new-symbolic", "New File");
    // ── Separator ──
    let sep = Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    // ── "Refresh" button ──
    let refresh_btn = context_menu_button("view-refresh-symbolic", "Refresh");

    menu_box.append(&new_folder_btn);
    menu_box.append(&new_file_btn);
    menu_box.append(&sep);
    menu_box.append(&refresh_btn);
    popover.set_child(Some(&menu_box));

    // Wire: New Folder
    {
        let popover_c = popover.clone();
        let cp = current_path.clone();
        let cb = content_box.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let cfg = config.clone();

        new_folder_btn.connect_clicked(move |_| {
            popover_c.popdown();
            show_name_input_dialog(
                &popover_c,
                "Create Folder",
                true,
                cp.clone(),
                cb.clone(),
                info.clone(),
                sel.clone(),
                cfg.clone(),
            );
        });
    }

    // Wire: New File
    {
        let popover_c = popover.clone();
        let cp = current_path.clone();
        let cb = content_box.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let cfg = config.clone();

        new_file_btn.connect_clicked(move |_| {
            popover_c.popdown();
            show_name_input_dialog(
                &popover_c,
                "Create File",
                false,
                cp.clone(),
                cb.clone(),
                info.clone(),
                sel.clone(),
                cfg.clone(),
            );
        });
    }

    // Wire: Refresh
    {
        let popover_c = popover.clone();
        let cp = current_path.clone();
        let cb = content_box.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let cfg = config.clone();

        refresh_btn.connect_clicked(move |_| {
            popover_c.popdown();
            refresh_content(&cb, cp.clone(), &info, sel.clone(), cfg.clone());
        });
    }

    popover
}

// ═══════════════════════════════════════════════
//  File Item Context Menu
// ═══════════════════════════════════════════════

/// Attaches a right-click context menu to a file/folder widget.
/// Provides "Open", "Rename", "Delete" options.
pub fn attach_file_context_menu(
    target: &impl IsA<Widget>,
    file_path: PathBuf,
    _file_name: String,
    current_path: Rc<RefCell<PathBuf>>,
    content_box: Box,
    inspector_info: Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
    let popover = Popover::builder()
        .css_classes(vec!["context-menu".to_string()])
        .build();

    popover.set_parent(target.as_ref());
    popover.set_has_arrow(true);

    // Ensure popover is unparented when the target widget is destroyed
    let popover_destroy = popover.clone();
    target.as_ref().connect_destroy(move |_| {
        popover_destroy.unparent();
    });

    let menu_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(4)
        .margin_end(4)
        .build();

    let open_btn = context_menu_button("document-open-symbolic", "Open");
    let rename_btn = context_menu_button("document-edit-symbolic", "Rename");
    let sep = Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let delete_btn = context_menu_button("user-trash-symbolic", "Delete");
    delete_btn.add_css_class("context-menu-danger");

    menu_box.append(&open_btn);
    menu_box.append(&rename_btn);
    menu_box.append(&sep);
    menu_box.append(&delete_btn);
    popover.set_child(Some(&menu_box));

    // Gesture
    let gesture = GestureClick::builder().button(3).build();

    let popover_c = popover.clone();
    gesture.connect_pressed(move |_gesture, _n, _x, _y| {
        popover_c.popup();
    });

    target.as_ref().add_controller(gesture);

    // Wire: Open
    {
        let file_path_c = file_path.clone();
        let popover_c = popover.clone();
        open_btn.connect_clicked(move |_| {
            popover_c.popdown();
            if let Err(e) = open::that(&file_path_c) {
                eprintln!("Failed to open file: {}", e);
            }
        });
    }

    // Wire: Rename
    {
        let file_path_c = file_path.clone();
        let popover_c = popover.clone();
        let cp = current_path.clone();
        let cb = content_box.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let cfg = config.clone();

        rename_btn.connect_clicked(move |_| {
            popover_c.popdown();
            show_rename_dialog(
                &popover_c,
                &file_path_c,
                cp.clone(),
                cb.clone(),
                info.clone(),
                sel.clone(),
                cfg.clone(),
            );
        });
    }

    // Wire: Delete
    {
        let file_path_c = file_path.clone();
        let popover_c = popover.clone();
        let cp = current_path.clone();
        let cb = content_box.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let cfg = config.clone();

        delete_btn.connect_clicked(move |_| {
            popover_c.popdown();
            // Perform deletion
            let result = if file_path_c.is_dir() {
                std::fs::remove_dir_all(&file_path_c)
            } else {
                std::fs::remove_file(&file_path_c)
            };
            match result {
                Ok(_) => {
                    refresh_content(&cb, cp.clone(), &info, sel.clone(), cfg.clone());
                }
                Err(e) => eprintln!("Failed to delete: {}", e),
            }
        });
    }
}

// ═══════════════════════════════════════════════
//  Dialogs
// ═══════════════════════════════════════════════

/// Shows a small inline popover to input a name for new file/folder creation.
fn show_name_input_dialog(
    parent_popover: &Popover,
    title: &str,
    is_dir: bool,
    current_path: Rc<RefCell<PathBuf>>,
    content_box: Box,
    inspector_info: Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
    let dialog = Popover::builder()
        .css_classes(vec!["context-menu".to_string()])
        .build();
    let parent_widget = match parent_popover.parent() {
        Some(w) => w,
        None => {
            eprintln!("[context_menu] No parent widget found for dialog");
            return;
        }
    };
    dialog.set_parent(&parent_widget);

    // Ensure dialog popover is unparented when its parent is destroyed
    let dialog_destroy = dialog.clone();
    parent_widget.connect_destroy(move |_| {
        dialog_destroy.unparent();
    });

    let vbox = Box::builder()
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

    let entry = GtkEntry::builder().placeholder_text("Name…").build();

    let create_btn = Button::builder()
        .label("Create")
        .css_classes(vec!["btn-primary".to_string()])
        .build();

    vbox.append(&label);
    vbox.append(&entry);
    vbox.append(&create_btn);
    dialog.set_child(Some(&vbox));

    // Clone everything before the first closure
    let dialog_c = dialog.clone();
    let entry_c = entry.clone();
    let cp1 = current_path.clone();
    let cb1 = content_box.clone();
    let info1 = inspector_info.clone();
    let sel1 = selected_file_path.clone();
    let cfg1 = config.clone();

    create_btn.connect_clicked(move |_| {
        let name = entry_c.text();
        if !name.is_empty() {
            let parent = cp1.borrow();
            let result = if is_dir {
                filesystem::create_directory(&parent, &name)
            } else {
                filesystem::create_file(&parent, &name)
            };
            match result {
                Ok(_) => {
                    dialog_c.popdown();
                    refresh_content(&cb1, cp1.clone(), &info1, sel1.clone(), cfg1.clone());
                }
                Err(e) => eprintln!("Creation failed: {}", e),
            }
        }
    });

    // Also allow Enter key
    let dialog_c2 = dialog.clone();
    let cp2 = current_path.clone();
    let cb2 = content_box.clone();
    let info2 = inspector_info.clone();
    let sel2 = selected_file_path.clone();
    let cfg2 = config.clone();
    entry.connect_activate(move |e| {
        let name = e.text();
        if !name.is_empty() {
            let parent = cp2.borrow();
            let result = if is_dir {
                filesystem::create_directory(&parent, &name)
            } else {
                filesystem::create_file(&parent, &name)
            };
            match result {
                Ok(_) => {
                    dialog_c2.popdown();
                    refresh_content(&cb2, cp2.clone(), &info2, sel2.clone(), cfg2.clone());
                }
                Err(e) => eprintln!("Creation failed: {}", e),
            }
        }
    });

    dialog.popup();
}

/// Shows a rename dialog popover.
fn show_rename_dialog(
    parent_popover: &Popover,
    file_path: &PathBuf,
    current_path: Rc<RefCell<PathBuf>>,
    content_box: Box,
    inspector_info: Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
    let dialog = Popover::builder()
        .css_classes(vec!["context-menu".to_string()])
        .build();
    let parent_widget = match parent_popover.parent() {
        Some(w) => w,
        None => {
            eprintln!("[context_menu] No parent widget found for rename dialog");
            return;
        }
    };
    dialog.set_parent(&parent_widget);

    // Ensure dialog popover is unparented when its parent is destroyed
    let dialog_destroy = dialog.clone();
    parent_widget.connect_destroy(move |_| {
        dialog_destroy.unparent();
    });

    let vbox = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let label = Label::builder()
        .label("Rename")
        .css_classes(vec!["context-menu-title".to_string()])
        .halign(Align::Start)
        .build();

    let old_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let entry = GtkEntry::builder().text(&old_name).build();

    let rename_btn = Button::builder()
        .label("Rename")
        .css_classes(vec!["btn-primary".to_string()])
        .build();

    vbox.append(&label);
    vbox.append(&entry);
    vbox.append(&rename_btn);
    dialog.set_child(Some(&vbox));

    let old_name1 = old_name.clone();
    let file_path_c1 = file_path.clone();
    let dialog_c1 = dialog.clone();
    let entry_c1 = entry.clone();
    let cp1 = current_path.clone();
    let cb1 = content_box.clone();
    let info1 = inspector_info.clone();
    let sel1 = selected_file_path.clone();
    let cfg1 = config.clone();

    rename_btn.connect_clicked(move |_| {
        let new_name = entry_c1.text();
        if !new_name.is_empty() && new_name.as_str() != old_name1 {
            if let Some(parent) = file_path_c1.parent() {
                let new_path = parent.join(new_name.as_str());
                match std::fs::rename(&file_path_c1, &new_path) {
                    Ok(_) => {
                        dialog_c1.popdown();
                        refresh_content(&cb1, cp1.clone(), &info1, sel1.clone(), cfg1.clone());
                    }
                    Err(e) => eprintln!("Rename failed: {}", e),
                }
            }
        }
    });

    let old_name2 = old_name.clone();
    let file_path_c2 = file_path.clone();
    let dialog_c2 = dialog.clone();
    let cp2 = current_path.clone();
    let cb2 = content_box.clone();
    let info2 = inspector_info.clone();
    let sel2 = selected_file_path.clone();
    let cfg2 = config.clone();

    entry.connect_activate(move |e| {
        let new_name = e.text();
        if !new_name.is_empty() && new_name.as_str() != old_name2 {
            if let Some(parent) = file_path_c2.parent() {
                let new_path = parent.join(new_name.as_str());
                match std::fs::rename(&file_path_c2, &new_path) {
                    Ok(_) => {
                        dialog_c2.popdown();
                        refresh_content(&cb2, cp2.clone(), &info2, sel2.clone(), cfg2.clone());
                    }
                    Err(e) => eprintln!("Rename failed: {}", e),
                }
            }
        }
    });

    dialog.popup();
}

// ═══════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════

/// Creates a styled context menu button with icon + label.
fn context_menu_button(icon_name: &str, label_text: &str) -> Button {
    let hbox = Box::builder()
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
