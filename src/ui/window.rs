use crate::filesystem;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, Label, Orientation, Paned, ScrolledWindow,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub fn build(app: &Application) {
    // Determine start path
    let start_path = dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    let current_path = Rc::new(RefCell::new(start_path));

    // Main Window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Diptych Project")
        .default_width(900)
        .default_height(600)
        .build();

    // --- Main Layout: Paned (Split View) ---
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .position(500) // Initial split position
        .build();

    // --- Left Panel: Navigation ---
    let nav_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .build();

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_width(300)
        .vexpand(true)
        .child(&nav_box)
        .build();

    // --- Right Panel: The "Inspector" ---
    let inspector_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .hexpand(true)
        .valign(Align::Center)
        .build();

    let info_label = Label::builder()
        .label("<span size='x-large' weight='bold'>Diptych</span>\n<span color='gray'>Select a file to inspect</span>")
        .use_markup(true)
        .justify(gtk4::Justification::Center)
        .wrap(true)
        .build();

    let action_button = Button::builder()
        .label("Open")
        .halign(Align::Center)
        .sensitive(false) // Disabled until selection
        .build();

    action_button.add_css_class("suggested-action"); // GTK theme accent style

    inspector_box.append(&info_label);
    inspector_box.append(&action_button);

    // Assemble Paned
    paned.set_start_child(Some(&scrolled_window));
    paned.set_end_child(Some(&inspector_box));

    window.set_child(Some(&paned));

    // Shared State for the Action Button
    let selected_file_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    // Action Button Logic
    let selected_file_clone = selected_file_path.clone();
    action_button.connect_clicked(move |_| {
        if let Some(path) = selected_file_clone.borrow().as_ref() {
            if let Err(e) = open::that(path) {
                eprintln!("Failed to open file: {}", e);
            } else {
                println!("Opening via Inspector: {:?}", path);
            }
        }
    });

    // Render Initial State
    refresh_ui(
        &nav_box,
        current_path,
        &window,
        &info_label,
        &action_button,
        selected_file_path,
    );

    window.present();
}

fn refresh_ui(
    container: &Box,
    current_path: Rc<RefCell<PathBuf>>,
    window: &ApplicationWindow,
    info_label: &Label,
    action_button: &Button,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
) {
    // Clear list
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let path = current_path.borrow();
    window.set_title(Some(&format!("Diptych - {}", path.to_string_lossy())));

    // Re-disable action button on nav change
    action_button.set_sensitive(false);
    *selected_file_path.borrow_mut() = None;
    info_label.set_markup("<span size='large'>Browsing...</span>");

    // "Go Up" Button
    if let Some(parent) = path.parent() {
        let parent_path = parent.to_path_buf();
        let up_button = Button::builder()
            .label("⬆️ Go Up")
            .halign(Align::Fill)
            .build();

        let path_clone = current_path.clone();
        let container_clone = container.clone();
        let window_clone = window.clone();
        let info_clone = info_label.clone();
        let action_clone = action_button.clone();
        let selected_clone = selected_file_path.clone();

        up_button.connect_clicked(move |_| {
            *path_clone.borrow_mut() = parent_path.clone();
            refresh_ui(
                &container_clone,
                path_clone.clone(),
                &window_clone,
                &info_clone,
                &action_clone,
                selected_clone.clone(),
            );
        });
        container.append(&up_button);
    }

    let files = filesystem::list_directory(&path);

    for entry in files {
        let label_text = if entry.is_dir {
            format!("📁 {}", entry.name)
        } else {
            format!("📄 {}", entry.name)
        };

        let button = Button::builder()
            .label(&label_text)
            .halign(Align::Fill)
            .has_frame(false)
            .build();

        if let Some(child) = button.child() {
            if let Some(label) = child.downcast_ref::<gtk4::Label>() {
                label.set_xalign(0.0);
            }
        }

        let entry_path = entry.path.clone();

        // Clones for closures
        let path_clone = current_path.clone();
        let container_clone = container.clone();
        let window_clone = window.clone();
        let info_clone = info_label.clone();
        let action_clone = action_button.clone();
        let selected_clone = selected_file_path.clone();

        if entry.is_dir {
            // Dirs: Navigate immmedeately
            button.connect_clicked(move |_| {
                *path_clone.borrow_mut() = entry_path.clone();
                refresh_ui(
                    &container_clone,
                    path_clone.clone(),
                    &window_clone,
                    &info_clone,
                    &action_clone,
                    selected_clone.clone(),
                );
            });
        } else {
            // Files: Select & Inspect
            let name_clone = entry.name.clone();
            button.connect_clicked(move |_| {
                // Update Inspector UI
                let markup = format!(
                    "<span size='xx-large' weight='bold'>{}</span>\n\n<span color='gray'>Type: File</span>\n<span color='gray'>Path: {}</span>",
                    name_clone,
                    entry_path.to_string_lossy()
                );
                info_clone.set_markup(&markup);

                // Update Action Button
                action_clone.set_sensitive(true);
                action_clone.set_label("Open File");
                *selected_clone.borrow_mut() = Some(entry_path.clone());
            });
        }

        container.append(&button);
    }
}
