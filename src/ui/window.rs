use crate::filesystem;
use crate::ui::widgets;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, Entry, Label, Orientation, Paned, Popover,
    ScrolledWindow, ToggleButton,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub fn build(app: &Application) {
    // Determine start path
    let start_path = dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    let current_path = Rc::new(RefCell::new(start_path));
    
    // State: Show Hidden Files (Default: false)
    let show_hidden = Rc::new(RefCell::new(false));

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
    let nav_settings_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(10)
        .margin_end(10)
        .build();

    let hidden_toggle = ToggleButton::builder()
        .label("Show Hidden Files")
        .active(false)
        .halign(Align::End)
        .build();
    
    nav_settings_box.append(&hidden_toggle);

    let nav_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .build();

    let left_panel_container = Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    
    left_panel_container.append(&nav_settings_box);

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_width(330)
        .vexpand(true)
        .child(&nav_box)
        .build();
    
    left_panel_container.append(&scrolled_window);

    // --- Right Panel: The "Inspector" & Actions ---
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

    // Inspector Action Buttons
    let open_button = Button::builder()
        .label("Open")
        .halign(Align::Center)
        .sensitive(false)
        .css_classes(vec!["suggested-action".to_string()])
        .build();

    // Creation Area
    let creation_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .halign(Align::Center)
        .margin_top(20)
        .build();

    let new_folder_btn = Button::builder().label("New Folder +").build();
    let new_file_btn = Button::builder().label("New File +").build();

    creation_box.append(&new_folder_btn);
    creation_box.append(&new_file_btn);

    inspector_box.append(&info_label);
    inspector_box.append(&open_button);
    inspector_box.append(&creation_box);

    // Assemble Paned
    paned.set_start_child(Some(&left_panel_container));
    paned.set_end_child(Some(&inspector_box));

    window.set_child(Some(&paned));

    // Shared State
    let selected_file_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    // --- Logic: Toggle Hidden Files ---
    let show_hidden_clone = show_hidden.clone();
    let nav_box_clone = nav_box.clone();
    let current_path_clone = current_path.clone();
    let window_clone = window.clone();
    let info_label_clone = info_label.clone();
    let open_button_clone = open_button.clone();
    let selected_file_clone = selected_file_path.clone();
    let show_hidden_ui_clone = show_hidden.clone(); // For recursive calls in refresh_ui

    hidden_toggle.connect_toggled(move |btn| {
        *show_hidden_clone.borrow_mut() = btn.is_active();
        refresh_ui(
            &nav_box_clone, 
            current_path_clone.clone(), 
            &window_clone, 
            &info_label_clone, 
            &open_button_clone, 
            selected_file_clone.clone(),
            show_hidden_clone.clone()
        );
    });

    // --- Logic: Open File ---
    let selected_file_clone = selected_file_path.clone();
    open_button.connect_clicked(move |_| {
        if let Some(path) = selected_file_clone.borrow().as_ref() {
            if let Err(e) = open::that(path) {
                eprintln!("Failed to open file: {}", e);
            } else {
                println!("Opening via Inspector: {:?}", path);
            }
        }
    });

    // --- Logic: Create New Folder ---
    setup_creation_popover(
        &new_folder_btn,
        "Folder Name...",
        current_path.clone(),
        nav_box.clone(),
        window.clone(),
        info_label.clone(),
        open_button.clone(),
        selected_file_path.clone(),
        true, // is_dir
        show_hidden.clone(),
    );

    // --- Logic: Create New File ---
    setup_creation_popover(
        &new_file_btn,
        "File Name...",
        current_path.clone(),
        nav_box.clone(),
        window.clone(),
        info_label.clone(),
        open_button.clone(),
        selected_file_path.clone(),
        false, // is_dir
        show_hidden.clone(),
    );

    // Render Initial State
    refresh_ui(
        &nav_box,
        current_path,
        &window,
        &info_label,
        &open_button,
        selected_file_path,
        show_hidden,
    );

    window.present();
}

// Helper to Attach Popover with Entry
fn setup_creation_popover(
    parent_btn: &Button,
    placeholder: &str,
    current_path: Rc<RefCell<PathBuf>>,
    nav_box: Box,
    window: ApplicationWindow,
    info_label: Label,
    open_button: Button,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    is_dir: bool,
    show_hidden: Rc<RefCell<bool>>,
) {
    let popover = Popover::builder().build();
    popover.set_parent(parent_btn);

    let box_container = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    let entry = Entry::builder().placeholder_text(placeholder).build();
    let create_confirm_btn = Button::builder().label("Create").build();

    box_container.append(&entry);
    box_container.append(&create_confirm_btn);
    // Set the box as the child (content) of the popover
    popover.set_child(Some(&box_container));

    let popover_clone = popover.clone();
    parent_btn.connect_clicked(move |_| {
        popover_clone.popup();
    });

    // Action Logic
    create_confirm_btn.connect_clicked(move |_| {
        let name = entry.text();
        if !name.is_empty() {
            let parent = current_path.borrow();
            let result = if is_dir {
                filesystem::create_directory(&parent, &name)
            } else {
                filesystem::create_file(&parent, &name)
            };

            match result {
                Ok(_) => {
                    println!("Created successfully: {}", name);
                    entry.set_text(""); // Clear
                    popover.popdown(); // Close
                    
                    // Refresh UI
                    refresh_ui(
                        &nav_box, 
                        current_path.clone(), 
                        &window, 
                        &info_label, 
                        &open_button, 
                        selected_file_path.clone(),
                        show_hidden.clone()
                    );
                }
                Err(e) => eprintln!("Creation failed: {}", e),
            }
        }
    });
}

fn refresh_ui(
    container: &Box,
    current_path: Rc<RefCell<PathBuf>>,
    window: &ApplicationWindow,
    info_label: &Label,
    action_button: &Button,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    show_hidden: Rc<RefCell<bool>>,
) {
    // Clear list
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let path = current_path.borrow();
    let is_hidden_visible = *show_hidden.borrow();
    window.set_title(Some(&format!("Diptych - {}", path.to_string_lossy())));

    // Re-disable action button on nav change
    action_button.set_sensitive(false);
    *selected_file_path.borrow_mut() = None;
    info_label.set_markup("<span size='large'>Browsing...</span>");

    // "Go Up" Button with Icon
    if let Some(parent) = path.parent() {
        let parent_path = parent.to_path_buf();
        // Custom Row for "Up"
        let up_button = widgets::create_file_row(".. (Go Up)", true);
        
        let path_clone = current_path.clone();
        let container_clone = container.clone();
        let window_clone = window.clone();
        let info_clone = info_label.clone();
        let action_clone = action_button.clone();
        let selected_clone = selected_file_path.clone();
        let show_hidden_clone = show_hidden.clone();

        up_button.connect_clicked(move |_| {
            *path_clone.borrow_mut() = parent_path.clone();
            refresh_ui(
                &container_clone,
                path_clone.clone(),
                &window_clone,
                &info_clone,
                &action_clone,
                selected_clone.clone(),
                show_hidden_clone.clone(),
            );
        });
        container.append(&up_button);
    }

    let files = filesystem::list_directory(&path, is_hidden_visible);

    for entry in files {
        // Use our new widget factory
        let button = widgets::create_file_row(&entry.name, entry.is_dir);

        let entry_path = entry.path.clone();

        // Clones for closures
        let path_clone = current_path.clone();
        let container_clone = container.clone();
        let window_clone = window.clone();
        let info_clone = info_label.clone();
        let action_clone = action_button.clone();
        let selected_clone = selected_file_path.clone();
        let show_hidden_clone = show_hidden.clone();

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
                    show_hidden_clone.clone(),
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
