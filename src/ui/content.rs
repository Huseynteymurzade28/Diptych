use crate::config::{AppConfig, GroupBy, ViewMode};
use crate::filesystem;
use crate::ui::widgets;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, FlowBox, Label};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Content Area Refresh
// ═══════════════════════════════════════════════

/// Refreshes the main content area (grid or list mode).
pub fn refresh_content(
    container: &Box,
    current_path: Rc<RefCell<PathBuf>>,
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

    let files = filesystem::list_directory(&path, cfg.show_hidden);

    // Group files
    let grouped = filesystem::group_entries(&files, &cfg.grouping);

    for (group_name, entries) in &grouped {
        // Group header (if grouping is active)
        if cfg.grouping != GroupBy::None && !group_name.is_empty() {
            let header = Label::builder()
                .label(group_name)
                .css_classes(vec!["group-header".to_string()])
                .halign(Align::Start)
                .build();
            container.append(&header);
        }

        match cfg.view_mode {
            ViewMode::Grid => {
                let flow = FlowBox::builder()
                    .selection_mode(gtk4::SelectionMode::None)
                    .homogeneous(false)
                    .row_spacing(6)
                    .column_spacing(6)
                    .margin_start(4)
                    .margin_end(4)
                    .margin_bottom(8)
                    .min_children_per_line(2)
                    .max_children_per_line(20)
                    .build();

                for entry in entries {
                    let card = widgets::create_file_card(entry, &cfg);
                    wire_content_click(
                        &card,
                        entry,
                        current_path.clone(),
                        container,
                        inspector_info,
                        selected_file_path.clone(),
                        config.clone(),
                    );
                    flow.insert(&card, -1);
                }
                container.append(&flow);
            }
            ViewMode::List => {
                for entry in entries {
                    let row = widgets::create_file_row(entry, &cfg);
                    wire_content_click(
                        &row,
                        entry,
                        current_path.clone(),
                        container,
                        inspector_info,
                        selected_file_path.clone(),
                        config.clone(),
                    );
                    container.append(&row);
                }
            }
        }
    }

    // Empty state
    if files.is_empty() {
        let empty = Label::builder()
            .label("This folder is empty")
            .css_classes(vec!["inspector-subtitle".to_string()])
            .halign(Align::Center)
            .valign(Align::Center)
            .vexpand(true)
            .build();
        container.append(&empty);
    }
}

// ═══════════════════════════════════════════════
//  Click Wiring
// ═══════════════════════════════════════════════

fn wire_content_click(
    btn: &Button,
    entry: &filesystem::Entry,
    current_path: Rc<RefCell<PathBuf>>,
    container: &Box,
    inspector_info: &Label,
    selected_file_path: Rc<RefCell<Option<PathBuf>>>,
    config: Rc<RefCell<AppConfig>>,
) {
    let entry_path = entry.path.clone();
    let is_dir = entry.is_dir;
    let name = entry.name.clone();
    let size_display = entry.size_display();
    let mod_display = entry.modified_display();

    let cp = current_path.clone();
    let cont = container.clone();
    let info = inspector_info.clone();
    let sel = selected_file_path.clone();
    let cfg = config.clone();

    btn.connect_clicked(move |_| {
        if is_dir {
            *cp.borrow_mut() = entry_path.clone();
            refresh_content(&cont, cp.clone(), &info, sel.clone(), cfg.clone());
        } else {
            info.set_label(&format!(
                "{}  •  {}  •  {}",
                name, size_display, mod_display
            ));
            *sel.borrow_mut() = Some(entry_path.clone());
            if let Err(e) = open::that(&entry_path) {
                eprintln!("Failed to open file: {}", e);
            }
        }
    });
}
