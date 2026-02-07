use crate::config::AppConfig;
use crate::filesystem;
use crate::ui::widgets;
use gtk4::prelude::*;
use gtk4::{
    Align, ApplicationWindow, Box, Button, Label, Orientation, Popover, ScrolledWindow, Separator,
    ToggleButton,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::ui::content::refresh_content;

// ═══════════════════════════════════════════════
//  Sidebar Construction
// ═══════════════════════════════════════════════

/// Builds the complete sidebar widget (toolbar + places + file browser).
pub fn build_sidebar(
    current_path: Rc<RefCell<PathBuf>>,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
    content_box: Box,
    content_scroll: ScrolledWindow,
    breadcrumb_label: Label,
    inspector_info: Label,
    window: ApplicationWindow,
    css_provider: gtk4::CssProvider,
) -> (Box, Box) {
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

    let nav_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(1)
        .margin_start(4)
        .margin_end(4)
        .build();

    let sidebar_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&nav_box)
        .build();
    sidebar.append(&sidebar_scroll);

    // ── Wire places shortcuts ──
    bind_places_logic(
        &places_box,
        current_path.clone(),
        nav_box.clone(),
        content_box.clone(),
        window.clone(),
        breadcrumb_label.clone(),
        inspector_info.clone(),
        selected_file_path.clone(),
        config.clone(),
    );

    // ── Wire creation popover ──
    setup_creation_popover(
        &new_item_btn,
        current_path.clone(),
        nav_box.clone(),
        content_box.clone(),
        window.clone(),
        breadcrumb_label.clone(),
        inspector_info.clone(),
        selected_file_path.clone(),
        config.clone(),
    );

    // ── Wire settings toggle ──
    {
        let content_scroll_c = content_scroll.clone();
        let config_c = config.clone();
        let css_c = css_provider;
        let nav_box_c = nav_box.clone();
        let content_box_c = content_box.clone();
        let window_c = window.clone();
        let breadcrumb_c = breadcrumb_label.clone();
        let inspector_info_c = inspector_info.clone();
        let selected_c = selected_file_path.clone();
        let current_path_c = current_path.clone();

        settings_toggle.connect_toggled(move |btn| {
            content_scroll_c.set_child(gtk4::Widget::NONE);

            if btn.is_active() {
                let on_change: Rc<dyn Fn()> = {
                    let nav_box_cc = nav_box_c.clone();
                    let window_cc = window_c.clone();
                    let breadcrumb_cc = breadcrumb_c.clone();
                    let inspector_info_cc = inspector_info_c.clone();
                    let selected_cc = selected_c.clone();
                    let current_path_cc = current_path_c.clone();
                    let config_cc = config_c.clone();
                    Rc::new(move || {
                        refresh_sidebar(
                            &nav_box_cc,
                            current_path_cc.clone(),
                            &window_cc,
                            &breadcrumb_cc,
                            &inspector_info_cc,
                            selected_cc.clone(),
                            config_cc.clone(),
                        );
                    })
                };
                let settings_panel = crate::ui::settings::build_settings_panel(
                    config_c.clone(),
                    css_c.clone(),
                    on_change,
                );
                let settings_scroll = ScrolledWindow::builder()
                    .hscrollbar_policy(gtk4::PolicyType::Never)
                    .vexpand(true)
                    .hexpand(true)
                    .child(&settings_panel)
                    .build();
                content_scroll_c.set_child(Some(&settings_scroll));
            } else {
                content_scroll_c.set_child(Some(&content_box_c));
                refresh_content(
                    &content_box_c,
                    current_path_c.clone(),
                    &inspector_info_c,
                    selected_c.clone(),
                    config_c.clone(),
                );
            }
        });
    }

    (sidebar, nav_box)
}

// ═══════════════════════════════════════════════
//  Places Shortcuts
// ═══════════════════════════════════════════════

fn bind_places_logic(
    container: &Box,
    current_path: Rc<RefCell<PathBuf>>,
    nav_box: Box,
    content_box: Box,
    window: ApplicationWindow,
    breadcrumb: Label,
    inspector_info: Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
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
            let path_clone = path.clone();

            let current_path = current_path.clone();
            let nav_box = nav_box.clone();
            let content_box = content_box.clone();
            let window = window.clone();
            let breadcrumb = breadcrumb.clone();
            let inspector_info = inspector_info.clone();
            let selected_file_path = selected_file_path.clone();
            let config = config.clone();

            btn.connect_clicked(move |_| {
                *current_path.borrow_mut() = path_clone.clone();
                refresh_all(
                    &nav_box,
                    &content_box,
                    current_path.clone(),
                    &window,
                    &breadcrumb,
                    &inspector_info,
                    selected_file_path.clone(),
                    config.clone(),
                );
            });
            container.append(&btn);
        }
    }
}

// ═══════════════════════════════════════════════
//  Creation Popover
// ═══════════════════════════════════════════════

fn setup_creation_popover(
    parent_btn: &Button,
    current_path: Rc<RefCell<PathBuf>>,
    nav_box: Box,
    content_box: Box,
    window: ApplicationWindow,
    breadcrumb: Label,
    inspector_info: Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
    let popover = Popover::builder().build();
    popover.set_parent(parent_btn);

    let pop_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let entry = gtk4::Entry::builder().placeholder_text("Name…").build();

    let btn_row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(Align::End)
        .build();

    let create_file_btn = Button::builder()
        .label("File")
        .css_classes(vec!["btn-secondary".to_string()])
        .build();

    let create_folder_btn = Button::builder()
        .label("Folder")
        .css_classes(vec!["btn-primary".to_string()])
        .build();

    btn_row.append(&create_file_btn);
    btn_row.append(&create_folder_btn);

    pop_box.append(&entry);
    pop_box.append(&btn_row);
    popover.set_child(Some(&pop_box));

    let popover_clone = popover.clone();
    parent_btn.connect_clicked(move |_| {
        popover_clone.popup();
    });

    let wire_creation = |is_dir: bool| {
        let entry = entry.clone();
        let popover = popover.clone();
        let current_path = current_path.clone();
        let nav_box = nav_box.clone();
        let content_box = content_box.clone();
        let window = window.clone();
        let breadcrumb = breadcrumb.clone();
        let inspector_info = inspector_info.clone();
        let selected_file_path = selected_file_path.clone();
        let config = config.clone();

        move |_: &Button| {
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
                        entry.set_text("");
                        popover.popdown();
                        refresh_all(
                            &nav_box,
                            &content_box,
                            current_path.clone(),
                            &window,
                            &breadcrumb,
                            &inspector_info,
                            selected_file_path.clone(),
                            config.clone(),
                        );
                    }
                    Err(e) => eprintln!("Creation failed: {}", e),
                }
            }
        }
    };

    create_folder_btn.connect_clicked(wire_creation(true));
    create_file_btn.connect_clicked(wire_creation(false));
}

// ═══════════════════════════════════════════════
//  Sidebar Refresh
// ═══════════════════════════════════════════════

/// Refreshes both sidebar and content.
pub fn refresh_all(
    nav_box: &Box,
    content_box: &Box,
    current_path: Rc<RefCell<PathBuf>>,
    window: &ApplicationWindow,
    breadcrumb: &Label,
    inspector_info: &Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
    refresh_sidebar(
        nav_box,
        current_path.clone(),
        window,
        breadcrumb,
        inspector_info,
        selected_file_path.clone(),
        config.clone(),
    );
    refresh_content(
        content_box,
        current_path,
        inspector_info,
        selected_file_path,
        config,
    );
}

/// Refreshes the sidebar file browser.
pub fn refresh_sidebar(
    container: &Box,
    current_path: Rc<RefCell<PathBuf>>,
    window: &ApplicationWindow,
    breadcrumb: &Label,
    inspector_info: &Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
    // Clear
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let path = current_path.borrow().clone();
    let cfg = config.borrow().clone();

    window.set_title(Some(&format!("Diptych — {}", path.to_string_lossy())));

    // Simplified breadcrumb
    let home = dirs::home_dir().unwrap_or_default();
    let display_path = if path.starts_with(&home) {
        format!("~/{}", path.strip_prefix(&home).unwrap_or(&path).display())
    } else {
        path.to_string_lossy().to_string()
    };
    breadcrumb.set_label(&display_path);

    // Reset inspector
    *selected_file_path.borrow_mut() = None;
    inspector_info.set_label("Select a file to inspect");

    // Go up button
    if let Some(parent) = path.parent() {
        let parent_path = parent.to_path_buf();
        let up_btn = widgets::create_go_up_row();

        let cp = current_path.clone();
        let cont = container.clone();
        let win = window.clone();
        let bc = breadcrumb.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let cfg_c = config.clone();

        up_btn.connect_clicked(move |_| {
            *cp.borrow_mut() = parent_path.clone();
            refresh_sidebar(
                &cont,
                cp.clone(),
                &win,
                &bc,
                &info,
                sel.clone(),
                cfg_c.clone(),
            );
        });
        container.append(&up_btn);
    }

    // List entries
    let files = filesystem::list_directory(&path, cfg.show_hidden);
    let dummy_config = AppConfig {
        icon_size: 48,
        show_file_size: false,
        show_modified_date: false,
        ..cfg.clone()
    };

    for entry in &files {
        let btn = widgets::create_file_row(entry, &dummy_config);
        let entry_path = entry.path.clone();

        let cp = current_path.clone();
        let cont = container.clone();
        let win = window.clone();
        let bc = breadcrumb.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let cfg_c = config.clone();
        let is_dir = entry.is_dir;
        let name = entry.name.clone();
        let size_display = entry.size_display();
        let mod_display = entry.modified_display();

        btn.connect_clicked(move |_| {
            if is_dir {
                *cp.borrow_mut() = entry_path.clone();
                refresh_sidebar(
                    &cont,
                    cp.clone(),
                    &win,
                    &bc,
                    &info,
                    sel.clone(),
                    cfg_c.clone(),
                );
            } else {
                info.set_label(&format!(
                    "{}  •  {}  •  {}",
                    name, size_display, mod_display
                ));
                *sel.borrow_mut() = Some(entry_path.clone());
            }
        });
        container.append(&btn);
    }
}
