use crate::filesystem;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub fn build(app: &Application) {
    // Determine start path (Home directory or fallback to current)
    let start_path = dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap());

    // Create a mutable shared state for the current path
    let current_path = Rc::new(RefCell::new(start_path));

    // Create a window and set the title
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Diptych File Manager")
        .default_width(800)
        .default_height(600)
        .build();

    // Create a vertical box to hold the file buttons
    let content_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Add scroll capability
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_width(360)
        .child(&content_box)
        .vexpand(true)
        .build();

    // Main layout box (could add a path bar later)
    let main_box = Box::builder().orientation(Orientation::Vertical).build();

    main_box.append(&scrolled_window);
    window.set_child(Some(&main_box));

    // Initial render
    refresh_ui(&content_box, current_path, &window);

    // Present window
    window.present();
}

fn refresh_ui(container: &Box, current_path: Rc<RefCell<PathBuf>>, window: &ApplicationWindow) {
    // Clear existing children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let path = current_path.borrow();
    window.set_title(Some(&format!("Diptych - {}", path.to_string_lossy())));

    // Add ".." button to go up if we are not at root
    if let Some(parent) = path.parent() {
        let parent_path = parent.to_path_buf();
        let up_button = Button::builder()
            .label(".. (Go Up)")
            .halign(gtk4::Align::Fill)
            .build();

        let path_clone = current_path.clone();
        let container_clone = container.clone();
        let window_clone = window.clone();

        up_button.connect_clicked(move |_| {
            *path_clone.borrow_mut() = parent_path.clone();
            refresh_ui(&container_clone, path_clone.clone(), &window_clone);
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
            .halign(gtk4::Align::Fill) // Fill the width
            .has_frame(false) // Flat look
            .build();

        // Align label inside button to the left
        if let Some(child) = button.child() {
            if let Some(label) = child.downcast_ref::<gtk4::Label>() {
                label.set_xalign(0.0);
            }
        }

        if entry.is_dir {
            let path_clone = current_path.clone();
            let container_clone = container.clone();
            let new_path = entry.path.clone();
            let window_clone = window.clone();

            button.connect_clicked(move |_| {
                *path_clone.borrow_mut() = new_path.clone();
                refresh_ui(&container_clone, path_clone.clone(), &window_clone);
            });
        } else {
            // File click logic: Open with default system app
            let file_path = entry.path.clone();
            button.connect_clicked(move |_| {
                if let Err(e) = open::that(&file_path) {
                    eprintln!("Failed to open file: {}", e);
                } else {
                    println!("Opening file: {:?}", file_path);
                }
            });
        }

        container.append(&button);
    }
}
