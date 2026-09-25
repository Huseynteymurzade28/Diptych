use crate::config::{GroupBy, ViewMode};
use crate::filesystem;
use crate::ui::state::AppState;
use crate::ui::{context_menu, graph_view, preview, tree_view, widgets};
use gtk4::prelude::*;
use gtk4::{Align, Button, FlowBox, Label};
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Content Area Refresh
// ═══════════════════════════════════════════════

/// Rebuilds the main content area for the current folder and view mode.
pub fn refresh_content(state: &Rc<AppState>) {
    let container = &state.content_box;
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let path = state.current_path();
    let cfg = state.config();

    match cfg.view_mode {
        ViewMode::Graph => {
            container.append(&graph_view::build_graph_view(&path));
            return;
        }
        ViewMode::Tree => {
            container.append(&tree_view::build_tree_view(state));
            return;
        }
        ViewMode::Grid | ViewMode::List => {}
    }

    let files = filesystem::list_directory(&path, cfg.show_hidden);
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

        if cfg.view_mode == ViewMode::Grid {
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
                wire_item(&card, entry, state);
                flow.insert(&card, -1);
            }
            container.append(&flow);
        } else {
            for entry in entries {
                let row = widgets::create_file_row(entry, &cfg);
                wire_item(&row, entry, state);
                container.append(&row);
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

fn wire_item(btn: &Button, entry: &filesystem::Entry, state: &Rc<AppState>) {
    // Click: select (or open, in single-click mode). Double click: open.
    {
        let entry = entry.clone();
        let state = state.clone();
        btn.connect_clicked(move |b| state.click(&entry, b.upcast_ref()));
    }
    if state.config.borrow().open_with == crate::config::OpenWith::DoubleClick {
        let gesture = gtk4::GestureClick::builder()
            .button(1)
            .propagation_phase(gtk4::PropagationPhase::Capture)
            .build();
        let entry = entry.clone();
        let state = state.clone();
        gesture.connect_pressed(move |_, n_press, _, _| {
            if n_press == 2 {
                state.activate(&entry);
            }
        });
        btn.add_controller(gesture);
    }
    // Keep the highlight when the view is rebuilt (theme/config changes).
    if state.selected().as_deref() == Some(entry.path.as_path()) {
        state.select(entry, Some(btn.upcast_ref()));
    }

    // Right-click context menu
    context_menu::attach_file_context_menu(btn, entry.path.clone(), state);

    // Hover tooltip with image preview for supported formats
    if preview::supports_preview(&entry.path) {
        let path = entry.path.clone();
        btn.set_has_tooltip(true);
        btn.connect_query_tooltip(move |_widget, _x, _y, _keyboard, tooltip| {
            match preview::build_tooltip_preview(&path) {
                Some(img) => {
                    tooltip.set_custom(Some(&img));
                    true
                }
                None => false,
            }
        });
    }
}
