use crate::config::{AppConfig, LayoutConfig, OpenWith, ViewMode};
use crate::filesystem::{self, Entry};
use crate::theme::ThemeManager;
use crate::ui::chrome::Chrome;
use crate::ui::file_view::FileView;
use crate::ui::{content, inspector, settings, sidebar};
use adw::prelude::*;
use gtk4::{Box, ScrolledWindow};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

// ═══════════════════════════════════════════════
//  Shared Application State
// ═══════════════════════════════════════════════
//
// One `AppState` per window, shared as `Rc<AppState>`.
//
// Rules:
//   • Views never mutate the current path directly — they call
//     `navigate_to()`, which refreshes *every* view, so the path bar,
//     title, sidebar, content and inspector can never disagree.
//   • File operations live here, so every menu / shortcut / button
//     triggers the exact same code path.

pub struct AppState {
    pub config: RefCell<AppConfig>,
    pub layout: RefCell<LayoutConfig>,
    current_path: RefCell<PathBuf>,
    /// Selected items, mirrored from the file view (or set by the tree view).
    selected: RefCell<Vec<PathBuf>>,
    /// The highlighted tree-view row, to un-highlight it on the next select.
    selected_widget: RefCell<glib::WeakRef<gtk4::Widget>>,
    back: RefCell<Vec<PathBuf>>,
    forward: RefCell<Vec<PathBuf>>,
    settings_visible: Cell<bool>,

    pub window: adw::ApplicationWindow,
    pub theme: Rc<ThemeManager>,
    pub chrome: Chrome,
    /// "files" → `file_view`; "other" → `content_scroll` (tree, graph, settings).
    pub content_stack: gtk4::Stack,
    pub file_view: Rc<FileView>,
    pub content_scroll: ScrolledWindow,
    pub content_box: Box,
    /// Place rows in the sidebar (highlighted when current).
    pub places: Box,
    pub inspector: Box,
}

/// Widgets the state needs to drive; built by `window::build`.
pub struct StateWidgets {
    pub window: adw::ApplicationWindow,
    pub theme: Rc<ThemeManager>,
    pub chrome: Chrome,
    pub content_stack: gtk4::Stack,
    pub file_view: Rc<FileView>,
    pub content_scroll: ScrolledWindow,
    pub content_box: Box,
    pub places: Box,
    pub inspector: Box,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        layout: LayoutConfig,
        start_path: PathBuf,
        w: StateWidgets,
    ) -> Rc<Self> {
        let state = Rc::new(Self {
            config: RefCell::new(config),
            layout: RefCell::new(layout),
            current_path: RefCell::new(start_path),
            selected: RefCell::new(vec![]),
            selected_widget: RefCell::new(glib::WeakRef::new()),
            back: RefCell::new(vec![]),
            forward: RefCell::new(vec![]),
            settings_visible: Cell::new(false),
            window: w.window,
            theme: w.theme,
            chrome: w.chrome,
            content_stack: w.content_stack,
            file_view: w.file_view,
            content_scroll: w.content_scroll,
            content_box: w.content_box,
            places: w.places,
            inspector: w.inspector,
        });
        install_actions(&state);
        state
    }

    // ─── Accessors ───

    pub fn current_path(&self) -> PathBuf {
        self.current_path.borrow().clone()
    }

    /// The selected item, if exactly one is selected.
    pub fn selected(&self) -> Option<PathBuf> {
        match self.selected.borrow().as_slice() {
            [one] => Some(one.clone()),
            _ => None,
        }
    }

    /// Every selected item.
    pub fn selection(&self) -> Vec<PathBuf> {
        self.selected.borrow().clone()
    }

    /// Snapshot of the config (so callers don't hold a borrow across callbacks).
    pub fn config(&self) -> AppConfig {
        self.config.borrow().clone()
    }

    // ─── Navigation ───

    /// The only way to change the current folder. Refreshes every view.
    pub fn navigate_to(self: &Rc<Self>, path: PathBuf) {
        if !path.is_dir() {
            eprintln!("Not a directory: {}", path.display());
            return;
        }
        let previous = self.current_path();
        if previous == path {
            return;
        }
        self.back.borrow_mut().push(previous);
        self.forward.borrow_mut().clear();
        self.set_path(path);
    }

    pub fn go_back(self: &Rc<Self>) {
        let target = self.back.borrow_mut().pop();
        if let Some(path) = target {
            self.forward.borrow_mut().push(self.current_path());
            self.set_path(path);
        }
    }

    pub fn go_forward(self: &Rc<Self>) {
        let target = self.forward.borrow_mut().pop();
        if let Some(path) = target {
            self.back.borrow_mut().push(self.current_path());
            self.set_path(path);
        }
    }

    /// Goes to the parent folder, with the folder we came from selected.
    pub fn go_up(self: &Rc<Self>) {
        let child = self.current_path();
        let Some(parent) = child.parent().map(Path::to_path_buf) else {
            return;
        };
        if !parent.is_dir() {
            return;
        }
        self.back.borrow_mut().push(child.clone());
        self.forward.borrow_mut().clear();
        *self.current_path.borrow_mut() = parent;
        *self.selected.borrow_mut() = vec![child];
        self.refresh();
    }

    fn set_path(self: &Rc<Self>, path: PathBuf) {
        *self.current_path.borrow_mut() = path;
        self.selected.borrow_mut().clear();
        self.refresh();
    }

    /// Opens a folder (navigates) or a file (default app).
    pub fn activate(self: &Rc<Self>, entry: &Entry) {
        if entry.is_dir {
            self.navigate_to(entry.path.clone());
        } else {
            self.open(&entry.path);
        }
    }

    /// Re-reads the current folder and redraws every view.
    pub fn refresh(self: &Rc<Self>) {
        // Drop selected files that vanished (deleted / renamed).
        self.selected.borrow_mut().retain(|p| p.exists());

        self.refresh_header();
        sidebar::refresh_places(self);
        if !self.settings_visible.get() {
            content::refresh_content(self);
        }
        inspector::refresh(self);
    }

    fn refresh_header(self: &Rc<Self>) {
        let path = self.current_path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".into());
        self.window.set_title(Some(&format!("{} — Diptych", name)));
        sidebar::refresh_path_bar(self);

        let cfg = self.config.borrow();
        set_action_state(&self.window, "toggle-hidden", cfg.show_hidden.to_variant());
        set_action_state(
            &self.window,
            "view-mode",
            view_mode_id(&cfg.view_mode).to_variant(),
        );
        set_action_enabled(&self.window, "back", !self.back.borrow().is_empty());
        set_action_enabled(&self.window, "forward", !self.forward.borrow().is_empty());
        set_action_enabled(&self.window, "go-up", path.parent().is_some());
        self.update_selection_actions();
    }

    /// Refreshes after a file operation. The file view follows the disk by
    /// itself (FileMonitor); other views are rebuilt.
    fn after_file_op(self: &Rc<Self>) {
        if self.file_view_active() && self.file_view.is_live() {
            self.selected.borrow_mut().retain(|p| p.exists());
            inspector::refresh(self);
        } else {
            self.refresh();
        }
    }

    /// Whether the grid / list file view is on screen.
    pub fn file_view_active(&self) -> bool {
        !self.settings_visible.get()
            && matches!(
                self.config.borrow().view_mode,
                ViewMode::Grid | ViewMode::List
            )
    }

    // ─── Selection ───

    /// Selects `entry` (tree view); `widget` (if any) gets the `.selected` highlight.
    pub fn select(self: &Rc<Self>, entry: &Entry, widget: Option<&gtk4::Widget>) {
        if let Some(old) = self.selected_widget.borrow().upgrade() {
            old.remove_css_class("selected");
        }
        if let Some(w) = widget {
            w.add_css_class("selected");
        }
        self.selected_widget.borrow().set(widget);
        self.set_selection(vec![entry.path.clone()]);
    }

    /// Replaces the selection; called by the views when theirs changes.
    pub fn set_selection(self: &Rc<Self>, paths: Vec<PathBuf>) {
        if *self.selected.borrow() == paths {
            return;
        }
        *self.selected.borrow_mut() = paths;
        self.update_selection_actions();
        inspector::refresh(self);
    }

    pub fn clear_selection(self: &Rc<Self>) {
        if let Some(old) = self.selected_widget.borrow().upgrade() {
            old.remove_css_class("selected");
        }
        self.file_view.unselect_all();
        self.set_selection(vec![]);
    }

    fn update_selection_actions(&self) {
        let n = self.selected.borrow().len();
        for name in ["open-selection", "trash-selection", "delete-selection"] {
            set_action_enabled(&self.window, name, n > 0);
        }
        set_action_enabled(&self.window, "rename-selection", n == 1);
    }

    // ─── Selection actions (menus, shortcuts, inspector) ───

    /// Opens the selection: a single folder is entered, files open in
    /// their default apps.
    pub fn open_selection(self: &Rc<Self>) {
        let paths = self.selection();
        if let [only] = paths.as_slice() {
            self.activate(&Entry::from_path(only));
            return;
        }
        for path in paths.iter().filter(|p| !p.is_dir()) {
            self.open(path);
        }
    }

    pub fn rename_selection(self: &Rc<Self>) {
        let Some(path) = self.selected() else { return };
        let anchor = if self.file_view_active() {
            self.file_view.anchor()
        } else {
            self.content_scroll.clone().upcast()
        };
        crate::ui::context_menu::show_rename_dialog(self, &anchor, &path);
    }

    pub fn trash_selection(self: &Rc<Self>) {
        self.trash_all(&self.selection());
    }

    /// What a click on an item does, per the "open with" preference.
    pub fn click(self: &Rc<Self>, entry: &Entry, widget: &gtk4::Widget) {
        match self.config.borrow().open_with {
            OpenWith::SingleClick => {}
            OpenWith::DoubleClick => {
                self.select(entry, Some(widget));
                return;
            }
        }
        self.activate(entry);
    }

    // ─── Layout ───

    /// Re-reads layout.toml and re-applies it (hot reload).
    pub fn reload_layout(self: &Rc<Self>) {
        let dir = crate::config::persistence::config_dir();
        match LayoutConfig::load(&dir) {
            Ok(layout) => {
                self.chrome.apply(&layout);
                *self.layout.borrow_mut() = layout;
                inspector::refresh(self);
                println!("[layout] Applied layout.toml");
            }
            Err(e) => eprintln!("[layout] layout.toml: {} (keeping previous layout)", e),
        }
    }

    // ─── Config ───

    /// Applies a config change, persists it and refreshes the UI.
    pub fn update_config(self: &Rc<Self>, f: impl FnOnce(&mut AppConfig)) {
        {
            let mut cfg = self.config.borrow_mut();
            f(&mut cfg);
            cfg.save();
        }
        self.refresh();
    }

    pub fn set_view_mode(self: &Rc<Self>, mode: ViewMode) {
        self.update_config(|cfg| cfg.view_mode = mode);
    }

    pub fn cycle_view_mode(self: &Rc<Self>) {
        self.update_config(|cfg| {
            cfg.view_mode = match cfg.view_mode {
                ViewMode::Grid => ViewMode::List,
                ViewMode::List => ViewMode::Graph,
                ViewMode::Graph => ViewMode::Tree,
                ViewMode::Tree => ViewMode::Grid,
            };
        });
    }

    // ─── Settings panel ───

    /// Swaps the content area between the file view and the settings panel.
    pub fn set_settings_visible(self: &Rc<Self>, visible: bool) {
        self.settings_visible.set(visible);
        set_action_state(&self.window, "show-settings", visible.to_variant());

        if visible {
            let panel = settings::build_settings_panel(self);
            let scroll = ScrolledWindow::builder()
                .hscrollbar_policy(gtk4::PolicyType::Never)
                .vexpand(true)
                .hexpand(true)
                .child(&panel)
                .build();
            self.content_scroll.set_child(Some(&scroll));
            self.file_view.stop();
            self.content_stack.set_visible_child_name("other");
        } else {
            self.content_scroll.set_child(Some(&self.content_box));
            content::refresh_content(self);
        }
    }

    // ─── File operations ───

    pub fn open(&self, path: &Path) {
        if let Err(e) = open::that(path) {
            eprintln!("Failed to open {}: {}", path.display(), e);
        }
    }

    /// Creates a file or folder named `name` in the current folder.
    pub fn create(self: &Rc<Self>, name: &str, is_dir: bool) -> Result<(), String> {
        let name = name.trim();
        validate_name(name)?;
        let parent = self.current_path();
        let result = if is_dir {
            filesystem::create_directory(&parent, name)
        } else {
            filesystem::create_file(&parent, name)
        };
        let created = result.map_err(|e| e.to_string())?;
        // Select the new item once it shows up.
        *self.selected.borrow_mut() = vec![created];
        self.after_file_op();
        Ok(())
    }

    pub fn rename(self: &Rc<Self>, path: &Path, new_name: &str) -> Result<(), String> {
        let new_name = new_name.trim();
        validate_name(new_name)?;
        let parent = path.parent().ok_or("Cannot rename the root folder")?;
        let new_path = parent.join(new_name);
        if new_path == path {
            return Ok(());
        }
        if new_path.exists() {
            return Err(format!("“{}” already exists", new_name));
        }
        std::fs::rename(path, &new_path).map_err(|e| e.to_string())?;
        for selected in self.selected.borrow_mut().iter_mut() {
            if selected == path {
                *selected = new_path.clone();
            }
        }
        self.after_file_op();
        Ok(())
    }

    pub fn trash(self: &Rc<Self>, path: &Path) {
        self.trash_all(&[path.to_path_buf()]);
    }

    fn trash_all(self: &Rc<Self>, paths: &[PathBuf]) {
        if paths.is_empty() {
            return;
        }
        for path in paths {
            if let Err(e) = filesystem::move_to_trash(path) {
                eprintln!("Failed to move {} to trash: {}", path.display(), e);
            }
        }
        self.after_file_op();
    }

    /// Asks for confirmation, then deletes `paths` permanently.
    pub fn confirm_delete_all(self: &Rc<Self>, paths: Vec<PathBuf>) {
        let (message, detail) = match paths.as_slice() {
            [] => return,
            [one] => (
                format!(
                    "Permanently delete “{}”?",
                    one.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                ),
                "This item will be deleted immediately. You can’t undo this action.",
            ),
            many => (
                format!("Permanently delete {} items?", many.len()),
                "These items will be deleted immediately. You can’t undo this action.",
            ),
        };
        let dialog = gtk4::AlertDialog::builder()
            .modal(true)
            .message(message)
            .detail(detail)
            .buttons(["Cancel", "Delete"])
            .cancel_button(0)
            .default_button(0)
            .build();

        let state = self.clone();
        dialog.choose(Some(&self.window), gio::Cancellable::NONE, move |choice| {
            if choice != Ok(1) {
                return;
            }
            for path in &paths {
                if let Err(e) = filesystem::delete_permanently(path) {
                    eprintln!("Failed to delete {}: {}", path.display(), e);
                }
            }
            state.after_file_op();
        });
    }
}

// ═══════════════════════════════════════════════
//  Window Actions (win.*)
// ═══════════════════════════════════════════════

fn install_actions(state: &Rc<AppState>) {
    let simple = |name: &str, f: fn(&Rc<AppState>)| {
        let s = state.clone();
        let action = gio::SimpleAction::new(name, None);
        action.connect_activate(move |_, _| f(&s));
        state.window.add_action(&action);
    };

    simple("back", |s| s.go_back());
    simple("forward", |s| s.go_forward());
    simple("go-up", |s| s.go_up());
    simple("refresh", |s| s.refresh());
    simple("cycle-view", |s| s.cycle_view_mode());
    simple("about", |s| show_about(&s.window));
    simple("open-selection", |s| s.open_selection());
    simple("rename-selection", |s| s.rename_selection());
    simple("trash-selection", |s| s.trash_selection());
    simple("delete-selection", |s| s.confirm_delete_all(s.selection()));
    simple("select-all", |s| {
        if s.file_view_active() {
            s.file_view.select_all();
        }
    });
    simple("unselect-all", |s| s.clear_selection());

    // Stateful boolean toggles (drive check marks / toggle buttons).
    let toggle = |name: &str, initial: bool, f: fn(&Rc<AppState>, bool)| {
        let s = state.clone();
        let action = gio::SimpleAction::new_stateful(name, None, &initial.to_variant());
        action.connect_change_state(move |_, value| {
            if let Some(v) = value.and_then(|v| v.get::<bool>()) {
                f(&s, v);
            }
        });
        state.window.add_action(&action);
    };

    toggle(
        "toggle-hidden",
        state.config.borrow().show_hidden,
        |s, v| s.update_config(|cfg| cfg.show_hidden = v),
    );
    toggle("show-settings", false, |s, v| s.set_settings_visible(v));

    // Radio-style: the view switcher buttons target "grid", "list", …
    let view_mode = gio::SimpleAction::new_stateful(
        "view-mode",
        Some(glib::VariantTy::STRING),
        &view_mode_id(&state.config.borrow().view_mode).to_variant(),
    );
    {
        let s = state.clone();
        view_mode.connect_change_state(move |_, value| {
            if let Some(mode) = value
                .and_then(|v| v.get::<String>())
                .and_then(|id| view_mode_from_id(&id))
            {
                s.set_view_mode(mode);
            }
        });
    }
    state.window.add_action(&view_mode);
}

/// Updates a stateful action without triggering its handler.
fn set_action_state(window: &adw::ApplicationWindow, name: &str, value: glib::Variant) {
    if let Some(action) = window
        .lookup_action(name)
        .and_downcast::<gio::SimpleAction>()
    {
        if action.state().as_ref() != Some(&value) {
            action.set_state(&value);
        }
    }
}

fn set_action_enabled(window: &adw::ApplicationWindow, name: &str, enabled: bool) {
    if let Some(action) = window
        .lookup_action(name)
        .and_downcast::<gio::SimpleAction>()
    {
        action.set_enabled(enabled);
    }
}

fn show_about(window: &adw::ApplicationWindow) {
    adw::AboutDialog::builder()
        .application_name("Diptych")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("A deeply customizable GTK4 file manager built with Rust.")
        .website("https://github.com/Huseynteymurzade28/Diptych")
        .issue_url("https://github.com/Huseynteymurzade28/Diptych/issues")
        .license_type(gtk4::License::MitX11)
        .build()
        .present(Some(window));
}

pub fn view_mode_id(mode: &ViewMode) -> &'static str {
    match mode {
        ViewMode::Grid => "grid",
        ViewMode::List => "list",
        ViewMode::Graph => "graph",
        ViewMode::Tree => "tree",
    }
}

fn view_mode_from_id(id: &str) -> Option<ViewMode> {
    Some(match id {
        "grid" => ViewMode::Grid,
        "list" => ViewMode::List,
        "graph" => ViewMode::Graph,
        "tree" => ViewMode::Tree,
        _ => return None,
    })
}

/// Rejects names that would escape the folder or can't exist on disk.
pub fn validate_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Name can’t be empty".into());
    }
    if name == "." || name == ".." {
        return Err("“.” and “..” are reserved names".into());
    }
    if name.contains('/') || name.contains('\0') {
        return Err("Name can’t contain “/”".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_name;

    #[test]
    fn accepts_normal_names() {
        assert!(validate_name("notes.txt").is_ok());
        assert!(validate_name("çalışma klasörü").is_ok());
        assert!(validate_name(".hidden").is_ok());
    }

    #[test]
    fn rejects_bad_names() {
        assert!(validate_name("").is_err());
        assert!(validate_name("   ").is_err());
        assert!(validate_name(".").is_err());
        assert!(validate_name("..").is_err());
        assert!(validate_name("a/b").is_err());
        assert!(validate_name("../escape").is_err());
    }
}
