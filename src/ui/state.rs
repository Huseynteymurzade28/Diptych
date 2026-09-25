use crate::config::{AppConfig, LayoutConfig, OpenWith, ViewMode};
use crate::filesystem::{self, Entry};
use crate::theme::ThemeManager;
use crate::ui::chrome::Chrome;
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
    selected: RefCell<Option<PathBuf>>,
    /// The highlighted item widget, to un-highlight it on the next select.
    selected_widget: RefCell<glib::WeakRef<gtk4::Widget>>,
    back: RefCell<Vec<PathBuf>>,
    forward: RefCell<Vec<PathBuf>>,
    settings_visible: Cell<bool>,

    pub window: adw::ApplicationWindow,
    pub theme: Rc<ThemeManager>,
    pub chrome: Chrome,
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
            selected: RefCell::new(None),
            selected_widget: RefCell::new(glib::WeakRef::new()),
            back: RefCell::new(vec![]),
            forward: RefCell::new(vec![]),
            settings_visible: Cell::new(false),
            window: w.window,
            theme: w.theme,
            chrome: w.chrome,
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

    pub fn selected(&self) -> Option<PathBuf> {
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

    pub fn go_up(self: &Rc<Self>) {
        let parent = self.current_path.borrow().parent().map(Path::to_path_buf);
        if let Some(parent) = parent {
            self.navigate_to(parent);
        }
    }

    fn set_path(self: &Rc<Self>, path: PathBuf) {
        *self.current_path.borrow_mut() = path;
        *self.selected.borrow_mut() = None;
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
        // Drop the selection if the file vanished (deleted / renamed).
        let gone = self.selected.borrow().as_ref().is_some_and(|p| !p.exists());
        if gone {
            *self.selected.borrow_mut() = None;
        }

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
    }

    // ─── Selection ───

    /// Selects `entry`; `widget` (if any) gets the `.selected` highlight.
    pub fn select(self: &Rc<Self>, entry: &Entry, widget: Option<&gtk4::Widget>) {
        if let Some(old) = self.selected_widget.borrow().upgrade() {
            old.remove_css_class("selected");
        }
        if let Some(w) = widget {
            w.add_css_class("selected");
        }
        self.selected_widget.borrow().set(widget);
        *self.selected.borrow_mut() = Some(entry.path.clone());
        inspector::refresh(self);
    }

    pub fn clear_selection(self: &Rc<Self>) {
        if self.selected.borrow().is_none() {
            return;
        }
        if let Some(old) = self.selected_widget.borrow().upgrade() {
            old.remove_css_class("selected");
        }
        *self.selected.borrow_mut() = None;
        inspector::refresh(self);
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
        result.map_err(|e| e.to_string())?;
        self.refresh();
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
        if self.selected.borrow().as_deref() == Some(path) {
            *self.selected.borrow_mut() = Some(new_path);
        }
        self.refresh();
        Ok(())
    }

    pub fn trash(self: &Rc<Self>, path: &Path) {
        match filesystem::move_to_trash(path) {
            Ok(_) => self.refresh(),
            Err(e) => eprintln!("Failed to move to trash: {}", e),
        }
    }

    /// Asks for confirmation, then deletes `path` permanently.
    pub fn confirm_delete_permanently(self: &Rc<Self>, path: &Path) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let dialog = gtk4::AlertDialog::builder()
            .modal(true)
            .message(format!("Permanently delete “{}”?", name))
            .detail("This item will be deleted immediately. You can’t undo this action.")
            .buttons(["Cancel", "Delete"])
            .cancel_button(0)
            .default_button(0)
            .build();

        let state = self.clone();
        let path = path.to_path_buf();
        dialog.choose(Some(&self.window), gio::Cancellable::NONE, move |choice| {
            if choice != Ok(1) {
                return;
            }
            match filesystem::delete_permanently(&path) {
                Ok(_) => state.refresh(),
                Err(e) => eprintln!("Failed to delete: {}", e),
            }
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
