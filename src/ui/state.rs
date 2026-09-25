use crate::config::{AppConfig, ViewMode};
use crate::core::Theme;
use crate::filesystem::{self, Entry};
use crate::ui::{content, settings, sidebar};
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Button, CssProvider, Label, ScrolledWindow};
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
//     `navigate_to()`, which refreshes *every* view, so the breadcrumb,
//     title, sidebar and content can never disagree.
//   • File operations live here, so every menu / shortcut / button
//     triggers the exact same code path.

pub struct AppState {
    pub config: RefCell<AppConfig>,
    current_path: RefCell<PathBuf>,
    selected: RefCell<Option<PathBuf>>,
    settings_visible: Cell<bool>,

    pub window: ApplicationWindow,
    pub css_provider: CssProvider,
    pub content_scroll: ScrolledWindow,
    pub content_box: Box,
    pub nav_box: Box,
    pub breadcrumb: Label,
    pub inspector_info: Label,
    pub view_toggle_btn: Button,
}

/// Widgets the state needs to drive; built by `window::build`.
pub struct StateWidgets {
    pub window: ApplicationWindow,
    pub css_provider: CssProvider,
    pub content_scroll: ScrolledWindow,
    pub content_box: Box,
    pub nav_box: Box,
    pub breadcrumb: Label,
    pub inspector_info: Label,
    pub view_toggle_btn: Button,
}

impl AppState {
    pub fn new(config: AppConfig, start_path: PathBuf, w: StateWidgets) -> Rc<Self> {
        let state = Rc::new(Self {
            config: RefCell::new(config),
            current_path: RefCell::new(start_path),
            selected: RefCell::new(None),
            settings_visible: Cell::new(false),
            window: w.window,
            css_provider: w.css_provider,
            content_scroll: w.content_scroll,
            content_box: w.content_box,
            nav_box: w.nav_box,
            breadcrumb: w.breadcrumb,
            inspector_info: w.inspector_info,
            view_toggle_btn: w.view_toggle_btn,
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
        *self.current_path.borrow_mut() = path;
        self.clear_selection();
        self.refresh();
    }

    pub fn go_up(self: &Rc<Self>) {
        let parent = self.current_path.borrow().parent().map(Path::to_path_buf);
        if let Some(parent) = parent {
            self.navigate_to(parent);
        }
    }

    /// Re-reads the current folder and redraws header, sidebar and content.
    pub fn refresh(self: &Rc<Self>) {
        // Drop the selection if the file vanished (deleted / renamed).
        let gone = self.selected.borrow().as_ref().is_some_and(|p| !p.exists());
        if gone {
            self.clear_selection();
        }

        self.refresh_header();
        sidebar::refresh_sidebar(self);
        if !self.settings_visible.get() {
            content::refresh_content(self);
        }
    }

    fn refresh_header(&self) {
        let path = self.current_path();
        self.window
            .set_title(Some(&format!("Diptych — {}", path.to_string_lossy())));

        let home = dirs::home_dir().unwrap_or_default();
        let display_path = match path.strip_prefix(&home) {
            Ok(rel) if rel.as_os_str().is_empty() => "~".to_string(),
            Ok(rel) => format!("~/{}", rel.display()),
            Err(_) => path.to_string_lossy().to_string(),
        };
        self.breadcrumb.set_label(&display_path);

        let cfg = self.config.borrow();
        self.view_toggle_btn
            .set_icon_name(view_mode_icon(&cfg.view_mode));
        set_action_state(&self.window, "toggle-hidden", cfg.show_hidden);
    }

    // ─── Selection ───

    pub fn select(&self, entry: &Entry) {
        *self.selected.borrow_mut() = Some(entry.path.clone());
        self.inspector_info.set_label(&format!(
            "{}  •  {}  •  {}",
            entry.name,
            entry.size_display(),
            entry.modified_display()
        ));
    }

    pub fn clear_selection(&self) {
        *self.selected.borrow_mut() = None;
        self.inspector_info.set_label("Select a file to inspect");
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

    pub fn apply_theme(&self, name: &str) {
        self.css_provider
            .load_from_data(&Theme::from_name(name).to_css());
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
        set_action_state(&self.window, "show-settings", visible);

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
}

/// Updates a stateful boolean action without triggering its handler.
fn set_action_state(window: &ApplicationWindow, name: &str, value: bool) {
    if let Some(action) = window
        .lookup_action(name)
        .and_downcast::<gio::SimpleAction>()
    {
        if action.state().and_then(|v| v.get::<bool>()) != Some(value) {
            action.set_state(&value.to_variant());
        }
    }
}

fn show_about(window: &ApplicationWindow) {
    gtk4::AboutDialog::builder()
        .transient_for(window)
        .modal(true)
        .program_name("Diptych")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("A modern GTK4 file manager built with Rust.")
        .website("https://github.com/Huseynteymurzade28/Diptych")
        .license_type(gtk4::License::MitX11)
        .build()
        .present();
}

pub fn view_mode_icon(mode: &ViewMode) -> &'static str {
    match mode {
        ViewMode::Grid => "view-grid-symbolic",
        ViewMode::List => "view-list-symbolic",
        ViewMode::Graph => "network-workgroup-symbolic",
        ViewMode::Tree => "view-list-tree-symbolic",
    }
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
