use crate::config::AppConfig;
use crate::filesystem;
use crate::ui::context_menu::name_submitter;
use crate::ui::state::AppState;
use crate::ui::widgets;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, Label, Orientation, Popover, ScrolledWindow, Separator, ToggleButton,
};
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Sidebar Construction
// ═══════════════════════════════════════════════

/// Builds the complete sidebar widget (toolbar + places + file browser).
pub fn build_sidebar(state: &Rc<AppState>) -> Box {
    let sidebar = Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(vec!["sidebar".to_string()])
        .width_request(200)
        .build();

    // ── Sidebar toolbar ──
    let sidebar_toolbar = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .margin_top(6)
        .margin_bottom(2)
        .margin_start(8)
        .margin_end(8)
        .css_classes(vec!["toolbar".to_string()])
        .build();

    let settings_toggle = ToggleButton::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text("Settings")
        .action_name("win.show-settings")
        .css_classes(vec!["toolbar-btn".to_string()])
        .build();

    let new_item_btn = Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("New File / Folder")
        .css_classes(vec!["toolbar-btn".to_string()])
        .build();

    let spacer = Box::builder().hexpand(true).build();

    sidebar_toolbar.append(&settings_toggle);
    sidebar_toolbar.append(&spacer);
    sidebar_toolbar.append(&new_item_btn);
    sidebar.append(&sidebar_toolbar);

    // ── Places section ──
    let places_title = Label::builder()
        .label("PLACES")
        .css_classes(vec!["sidebar-title".to_string()])
        .halign(Align::Start)
        .margin_top(8)
        .build();
    sidebar.append(&places_title);

    let places_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(1)
        .margin_start(4)
        .margin_end(4)
        .build();
    sidebar.append(&places_box);

    sidebar.append(
        &Separator::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(8)
            .margin_bottom(4)
            .margin_start(12)
            .margin_end(12)
            .build(),
    );

    // ── Current directory file list ──
    let sidebar_files_title = Label::builder()
        .label("BROWSER")
        .css_classes(vec!["sidebar-title".to_string()])
        .halign(Align::Start)
        .build();
    sidebar.append(&sidebar_files_title);

    let sidebar_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&state.nav_box)
        .build();
    sidebar.append(&sidebar_scroll);

    bind_places_logic(&places_box, state);
    setup_creation_popover(&new_item_btn, state);

    sidebar
}

// ═══════════════════════════════════════════════
//  Places Shortcuts
// ═══════════════════════════════════════════════

fn bind_places_logic(container: &Box, state: &Rc<AppState>) {
    let places = vec![
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
    ];

    for (name, icon, path_opt) in places {
        if let Some(path) = path_opt {
            let btn = widgets::create_place_row(name, icon);
            let state = state.clone();
            btn.connect_clicked(move |_| state.navigate_to(path.clone()));
            container.append(&btn);
        }
    }
}

// ═══════════════════════════════════════════════
//  Creation Popover
// ═══════════════════════════════════════════════

fn setup_creation_popover(parent_btn: &Button, state: &Rc<AppState>) {
    let popover = Popover::builder()
        .css_classes(vec!["context-menu".to_string()])
        .build();
    popover.set_parent(parent_btn);

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
        .css_classes(vec!["context-menu-title".to_string()])
        .halign(Align::Start)
        .build();

    let entry = gtk4::Entry::builder().placeholder_text("Name…").build();

    let btn_row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(Align::End)
        .build();

    let create_file_btn = Button::builder()
        .label("  File  ")
        .css_classes(vec![
            "btn-secondary".to_string(),
            "creation-btn".to_string(),
        ])
        .build();

    let create_folder_btn = Button::builder()
        .label("  Folder  ")
        .css_classes(vec!["btn-primary".to_string(), "creation-btn".to_string()])
        .build();

    btn_row.append(&create_file_btn);
    btn_row.append(&create_folder_btn);

    pop_box.append(&title_label);
    pop_box.append(&entry);
    pop_box.append(&btn_row);
    popover.set_child(Some(&pop_box));

    let popover_clone = popover.clone();
    parent_btn.connect_clicked(move |_| popover_clone.popup());

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

// ═══════════════════════════════════════════════
//  Sidebar Refresh
// ═══════════════════════════════════════════════

/// Refreshes the sidebar file browser for the current folder.
pub fn refresh_sidebar(state: &Rc<AppState>) {
    let container = &state.nav_box;
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let path = state.current_path();
    let cfg = state.config();

    // Go up row
    if path.parent().is_some() {
        let up_btn = widgets::create_go_up_row();
        up_btn.set_action_name(Some("win.go-up"));
        container.append(&up_btn);
    }

    // List entries (compact: no metadata columns)
    let files = filesystem::list_directory(&path, cfg.show_hidden);
    let row_config = AppConfig {
        icon_size: 48,
        show_file_size: false,
        show_modified_date: false,
        ..cfg
    };

    for entry in files {
        let btn = widgets::create_file_row(&entry, &row_config);
        let state = state.clone();
        btn.connect_clicked(move |_| {
            if entry.is_dir {
                state.navigate_to(entry.path.clone());
            } else {
                state.select(&entry);
            }
        });
        container.append(&btn);
    }
}
