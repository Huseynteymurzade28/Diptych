use crate::config::{AppConfig, GroupBy, ViewMode};
use crate::filesystem;
use crate::ui::{context_menu, graph_view, preview, tree_view, widgets};
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

    // Graph mode gets its own special view
    if cfg.view_mode == ViewMode::Graph {
        let graph = graph_view::build_graph_view(current_path.clone(), config.clone());
        container.append(&graph);
        return;
    }

    // Tree mode: hierarchical expand/collapse view
    if cfg.view_mode == ViewMode::Tree {
        let cp = current_path.clone();
        let cont = container.clone();
        let info = inspector_info.clone();
        let sel = selected_file_path.clone();
        let cfg_rc = config.clone();

        let on_navigate: Rc<dyn Fn(PathBuf)> = Rc::new(move |new_path: PathBuf| {
            *cp.borrow_mut() = new_path;
            refresh_content(
                &cont,
                cp.clone(),
                &info,
                sel.clone(),
                cfg_rc.clone(),
            );
        });

        let tree = tree_view::build_tree_view(
            current_path.clone(),
            config.clone(),
            inspector_info,
            selected_file_path.clone(),
            on_navigate,
        );
        container.append(&tree);
        return;
    }

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
            ViewMode::Graph => {
                // Graph mode is handled at the top of refresh_content
                unreachable!("Graph mode should be handled before grouping");
            }
            ViewMode::Tree => {
                // Tree mode is handled at the top of refresh_content
                unreachable!("Tree mode should be handled before grouping");
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

    // Left-click: navigate or open + show preview
    let entry_path_click = entry_path.clone();
    let info_click = info.clone();
    let sel_click = sel.clone();
    btn.connect_clicked(move |_| {
        if is_dir {
            *cp.borrow_mut() = entry_path_click.clone();
            refresh_content(
                &cont,
                cp.clone(),
                &info_click,
                sel_click.clone(),
                cfg.clone(),
            );
        } else {
            info_click.set_label(&format!(
                "{}  •  {}  •  {}",
                name, size_display, mod_display
            ));
            *sel_click.borrow_mut() = Some(entry_path_click.clone());
            if let Err(e) = open::that(&entry_path_click) {
                eprintln!("Failed to open file: {}", e);
            }
        }
    });

    // Right-click context menu (Rename, Delete, Open)
    context_menu::attach_file_context_menu(
        btn,
        entry.path.clone(),
        entry.name.clone(),
        current_path.clone(),
        container.clone(),
        inspector_info.clone(),
        selected_file_path.clone(),
        config.clone(),
    );

    // Hover tooltip with image preview for supported formats
    if preview::supports_preview(&entry.path) {
        let entry_path_tooltip = entry.path.clone();
        btn.set_has_tooltip(true);
        btn.connect_query_tooltip(move |_widget, _x, _y, _keyboard, tooltip| {
            if let Some(preview_img) = preview::build_tooltip_preview(&entry_path_tooltip) {
                tooltip.set_custom(Some(&preview_img));
                return true;
            }
            false
        });
    }
}
