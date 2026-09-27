use crate::config::ViewMode;
use crate::ui::state::AppState;
use crate::ui::{graph_view, tree_view};
use gtk4::prelude::*;
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Content Area Refresh
// ═══════════════════════════════════════════════
//
// Grid and list are the virtualized `FileView`; tree and graph are
// built into `content_box`, on the stack's "other" page.

/// Shows the current folder in the configured view mode.
pub fn refresh_content(state: &Rc<AppState>) {
    let mode = state.config.borrow().view_mode.clone();
    if matches!(mode, ViewMode::Grid | ViewMode::List) {
        state.content_stack.set_visible_child_name("files");
        state.file_view.show(state);
        return;
    }

    state.file_view.stop();
    state.content_stack.set_visible_child_name("other");
    let container = &state.content_box;
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    match mode {
        ViewMode::Graph => container.append(&graph_view::build_graph_view(&state.current_path())),
        ViewMode::Tree => container.append(&tree_view::build_tree_view(state)),
        ViewMode::Grid | ViewMode::List => unreachable!(),
    }
}
