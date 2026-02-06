use crate::filesystem;
use crate::ui::{themes, widgets};
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, CssProvider, Entry, Label, Orientation,
    Paned, Popover, ScrolledWindow, StyleContext, ToggleButton,
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

    // --- Theme Setup ---
    let css_provider = CssProvider::new();
    css_provider.load_from_data(themes::get_css("Tokyo Night")); // Default

    // Apply CSS provider to the default display
    if let Some(display) = gtk4::gdk::Display::default() {
        StyleContext::add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Main Window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Diptych Project")
        .default_width(1000) // Slightly wider for Sidebar
        .default_height(600)
        .build();

    // --- Main Layout: Paned (Split View) ---
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .position(280) // Initial split position (adjusted for sidebar)
        .build();

    // --- Left Panel Container ---
    let left_panel_container = Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(vec!["sidebar".to_string()]) // Apply sidebar theme
        .build();

    // 1. Toolbar (Hidden Toggle & Settings)
    let toolbar_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(5)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(10)
        .margin_end(10)
        .build();

    let hidden_toggle = ToggleButton::builder()
        .icon_name("view-reveal-symbolic") // Use icon instead of text for compactness
        .tooltip_text("Toggle Hidden Files")
        .active(false)
        .build();
    
    let settings_btn = Button::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text("Theme Settings")
        .build();

    let settings_popover = Popover::builder().build();
    setup_theme_popover(&settings_btn, &settings_popover, &css_provider);

    toolbar_box.append(&hidden_toggle);
    toolbar_box.append(&settings_btn);

    // 2. Places Sidebar (Static Shortcuts)
    let places_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(10)
        .margin_start(10)
        .margin_end(10)
        .build();

    // 3. Current Directory List
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
        .min_content_width(250)
        .vexpand(true)
        .child(&nav_box)
        .build();

    // Separator between Places and Files
    let separator = gtk4::Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(5)
        .margin_bottom(5)
        .build();

    left_panel_container.append(&toolbar_box);
    left_panel_container.append(&Label::builder().label("<b>Places</b>").use_markup(true).xalign(0.0).margin_start(12).build());
    left_panel_container.append(&places_box);
    left_panel_container.append(&separator);
    left_panel_container.append(&Label::builder().label("<b>Files</b>").use_markup(true).xalign(0.0).margin_start(12).build());
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
    
    // ... existing right panel code ...
    let info_label = Label::builder()
        .label("<span size='x-large' weight='bold'>Diptych</span>\n<span color='gray'>Select a file to inspect</span>")
        .use_markup(true)
        .justify(gtk4::Justification::Center)
        .wrap(true)
        .build();

    let open_button = Button::builder()
        .label("Open")
        .halign(Align::Center)
        .sensitive(false)
        .css_classes(vec!["suggested-action".to_string()])
        .build();

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
    // ... end existing right panel code ...

    // Assemble Paned
    paned.set_start_child(Some(&left_panel_container));
    paned.set_end_child(Some(&inspector_box));

    window.set_child(Some(&paned));

    // Shared State
    let selected_file_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    // --- Logic Wiring ---
    
    // Places logic needs access to refresh_ui params.
    // We need to pass closures to add_places_shortcuts or handle it differently.
    // Refactor: Places need to update current_path and trigger refresh.
    // The simple way: Store context in a struct or just re-bind closures (verbose but works).
    
    // Re-bind Places Shortcuts with Logic
    // Actually, I'll clear `places_box` and re-build it? No, Places are static.
    // But they need to trigger `refresh_ui`. 
    // So I should build places AFTER I have all the clones ready.
    
    // Let's reorganize the build order slightly to access clones.

    // --- Logic: Toggle Hidden ---
    let show_hidden_clone = show_hidden.clone();
    let nav_box_clone = nav_box.clone();
    let current_path_clone = current_path.clone();
    let window_clone = window.clone();
    let info_label_clone = info_label.clone();
    let open_button_clone = open_button.clone();
    let selected_file_clone = selected_file_path.clone();

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
    let selected_file_clone_2 = selected_file_path.clone();
    open_button.connect_clicked(move |_| {
         if let Some(path) = selected_file_clone_2.borrow().as_ref() {
             if let Err(e) = open::that(path) {
                 eprintln!("Failed to open file: {}", e);
             } else {
                 println!("Opening via Inspector: {:?}", path);
             }
         }
    });

    // --- Wiring Places Shortcuts ---
    bind_places_logic(
        &places_box,
        current_path.clone(),
        nav_box.clone(),
        window.clone(),
        info_label.clone(),
        open_button.clone(),
        selected_file_path.clone(),
        show_hidden.clone(),
    );

    // --- Logic: Creation ---
    setup_creation_popover(
        &new_folder_btn, "Folder Name...", current_path.clone(), nav_box.clone(), window.clone(), info_label.clone(), open_button.clone(), selected_file_path.clone(), true, show_hidden.clone()
    );
    setup_creation_popover(
        &new_file_btn, "File Name...", current_path.clone(), nav_box.clone(), window.clone(), info_label.clone(), open_button.clone(), selected_file_path.clone(), false, show_hidden.clone()
    );

    // Initial Render
    refresh_ui(&nav_box, current_path, &window, &info_label, &open_button, selected_file_path, show_hidden);

    window.present();
}

fn setup_theme_popover(btn: &Button, popover: &Popover, provider: &CssProvider) {
    popover.set_parent(btn);
    let box_container = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    let label = Label::builder().label("<b>Select Theme</b>").use_markup(true).margin_bottom(4).build();
    box_container.append(&label);

    for theme_name in themes::all_themes() {
        let theme_btn = Button::builder()
            .label(theme_name)
            .has_frame(false)
            .build();
        
        // Logic
        let provider_clone = provider.clone();
        let name = theme_name.to_string();
        theme_btn.connect_clicked(move |_| {
            provider_clone.load_from_data(themes::get_css(&name));
        });

        box_container.append(&theme_btn);
    }

    popover.set_child(Some(&box_container));

    let popover_clone = popover.clone();
    btn.connect_clicked(move |_| {
        popover_clone.popup();
    });
}

fn bind_places_logic(
    container: &Box,
    current_path: Rc<RefCell<PathBuf>>,
    nav_box: Box,
    window: ApplicationWindow,
    info_label: Label,
    open_button: Button,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    show_hidden: Rc<RefCell<bool>>,
) {
    let places = vec![
        ("Start", dirs::home_dir()),
        ("Desktop", dirs::desktop_dir()),
        ("Documents", dirs::document_dir()),
        ("Downloads", dirs::download_dir()),
        ("Pictures", dirs::picture_dir()),
        ("Music", dirs::audio_dir()),
        ("Videos", dirs::video_dir()),
    ];

    for (name, path_opt) in places {
        if let Some(path) = path_opt {
            let btn = widgets::create_file_row(name, true); // Reusing create_row for consistent look
            
            // Clean up icon for places if possible? 
            // widgets::create_file_row uses standard "folder" icon.
            // We could improve this later with specific icons (user-desktop, folder-documents etc).
            // For now, consistent style is fine.

            let path_clone = path.clone();
            
            // Clones
            let current_path = current_path.clone();
            let nav_box = nav_box.clone();
            let window = window.clone();
            let info_label = info_label.clone();
            let open_button = open_button.clone();
            let selected_file_path = selected_file_path.clone();
            let show_hidden = show_hidden.clone();

            btn.connect_clicked(move |_| {
                *current_path.borrow_mut() = path_clone.clone();
                refresh_ui(
                    &nav_box, 
                    current_path.clone(), 
                    &window, 
                    &info_label, 
                    &open_button, 
                    selected_file_path.clone(),
                    show_hidden.clone()
                );
            });
            container.append(&btn);
        }
    }
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
