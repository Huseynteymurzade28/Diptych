// ─── Theme Module ───
// Token-based theming: presets + `theme.toml` → CSS, `user.css` on top,
// both hot-reloaded from `~/.config/diptych/`.

mod color;
mod css;
mod model;

pub use model::{Theme, ThemeFile};

use gtk4::prelude::*;
use gtk4::CssProvider;
use model::{Presets, BUILTIN_PRESETS, DEFAULT_PRESET};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const THEME_FILE: &str = "theme.toml";
const USER_CSS: &str = "user.css";
const THEMES_DIR: &str = "themes";

/// Owns the theme + user CSS providers and keeps them in sync with disk.
pub struct ThemeManager {
    dir: PathBuf,
    theme_provider: CssProvider,
    user_provider: CssProvider,
    last_error: RefCell<Option<String>>,
    /// Kept alive so file events keep arriving.
    monitors: RefCell<Vec<gio::FileMonitor>>,
    pending_reload: RefCell<Option<glib::SourceId>>,
}

impl ThemeManager {
    /// Installs the providers on the default display, loads the theme and
    /// starts watching the config directory. `legacy_name` is the old
    /// `config.toml` theme name, used once to seed `theme.toml`.
    pub fn new(config_dir: &Path, legacy_name: &str) -> Rc<Self> {
        let manager = Rc::new(Self {
            dir: config_dir.to_path_buf(),
            theme_provider: CssProvider::new(),
            user_provider: CssProvider::new(),
            last_error: RefCell::new(None),
            monitors: RefCell::new(vec![]),
            pending_reload: RefCell::new(None),
        });

        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &manager.theme_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            // USER priority: user.css wins over everything else.
            gtk4::style_context_add_provider_for_display(
                &display,
                &manager.user_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }
        for (provider, name) in [
            (&manager.theme_provider, "generated theme CSS"),
            (&manager.user_provider, USER_CSS),
        ] {
            provider.connect_parsing_error(move |_, section, error| {
                let loc = section.start_location();
                eprintln!(
                    "[theme] {}:{}:{}: {}",
                    name,
                    loc.lines() + 1,
                    loc.line_chars() + 1,
                    error
                );
            });
        }

        if let Err(e) = seed_theme_file(&manager.theme_path(), legacy_name) {
            eprintln!("[theme] Could not create {}: {}", THEME_FILE, e);
        }
        manager.reload();
        manager.watch();
        manager
    }

    pub fn theme_path(&self) -> PathBuf {
        self.dir.join(THEME_FILE)
    }

    /// The last error from loading `theme.toml`, if the current load failed.
    pub fn last_error(&self) -> Option<String> {
        self.last_error.borrow().clone()
    }

    /// Re-reads `theme.toml` and `user.css`. On a theme error the previous
    /// theme stays active, so a half-typed edit never breaks the UI.
    pub fn reload(&self) {
        match self.load_theme() {
            Ok(theme) => match css::generate(&theme) {
                Ok(css) => {
                    self.theme_provider.load_from_data(&css);
                    adw::StyleManager::default().set_color_scheme(if theme.dark {
                        adw::ColorScheme::ForceDark
                    } else {
                        adw::ColorScheme::ForceLight
                    });
                    println!("[theme] Applied “{}”", theme.name);
                    *self.last_error.borrow_mut() = None;
                }
                Err(e) => self.report(e),
            },
            Err(e) => self.report(e),
        }

        let user_css = std::fs::read_to_string(self.dir.join(USER_CSS)).unwrap_or_default();
        self.user_provider.load_from_data(&user_css);
    }

    fn report(&self, error: String) {
        // Editors often save twice in a row; don't repeat the same message.
        if self.last_error.borrow().as_ref() == Some(&error) {
            return;
        }
        eprintln!("[theme] {}: {} (keeping previous theme)", THEME_FILE, error);
        *self.last_error.borrow_mut() = Some(error);
    }

    fn load_theme(&self) -> Result<Theme, String> {
        let src = std::fs::read_to_string(self.theme_path()).unwrap_or_default();
        let file = ThemeFile::parse(&src)?;
        let user_dir = self.dir.join(THEMES_DIR);
        Theme::resolve(
            &file,
            &Presets {
                user_dir: Some(&user_dir),
            },
        )
    }

    /// `(id, display name)` of every selectable preset: built-ins, then
    /// the user's `themes/*.toml`.
    pub fn available(&self) -> Vec<(String, String)> {
        let mut list: Vec<(String, String)> = BUILTIN_PRESETS
            .iter()
            .map(|(id, src)| (id.to_string(), preset_name(src).unwrap_or(id.to_string())))
            .collect();
        if let Ok(entries) = std::fs::read_dir(self.dir.join(THEMES_DIR)) {
            let mut user: Vec<(String, String)> = entries
                .flatten()
                .filter_map(|e| {
                    let path = e.path();
                    let id = path.file_stem()?.to_str()?.to_string();
                    (path.extension()? == "toml").then_some(())?;
                    let name = std::fs::read_to_string(&path)
                        .ok()
                        .and_then(|src| preset_name(&src))
                        .unwrap_or_else(|| id.clone());
                    Some((id, name))
                })
                .collect();
            user.sort();
            list.retain(|(id, _)| !user.iter().any(|(uid, _)| uid == id));
            list.extend(user);
        }
        list
    }

    /// The `base` currently set in `theme.toml`.
    pub fn base(&self) -> String {
        std::fs::read_to_string(self.theme_path())
            .ok()
            .and_then(|src| ThemeFile::parse(&src).ok())
            .and_then(|f| f.base)
            .unwrap_or_else(|| DEFAULT_PRESET.to_string())
    }

    /// Switches the preset by editing `base` in `theme.toml`, keeping the
    /// user's comments and overrides intact.
    pub fn set_base(&self, id: &str) -> Result<(), String> {
        let path = self.theme_path();
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        std::fs::write(&path, with_base(&src, id)?).map_err(|e| e.to_string())?;
        self.reload();
        Ok(())
    }

    // ─── Hot reload ───

    fn watch(self: &Rc<Self>) {
        let _ = std::fs::create_dir_all(self.dir.join(THEMES_DIR));
        for dir in [self.dir.clone(), self.dir.join(THEMES_DIR)] {
            let file = gio::File::for_path(&dir);
            match file.monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
            {
                Ok(monitor) => {
                    let weak = Rc::downgrade(self);
                    monitor.connect_changed(move |_, file, _, _| {
                        let relevant = file.basename().is_some_and(|name| {
                            name == Path::new(THEME_FILE)
                                || name == Path::new(USER_CSS)
                                || name.extension().is_some_and(|e| e == "toml")
                        });
                        if let (true, Some(manager)) = (relevant, weak.upgrade()) {
                            manager.schedule_reload();
                        }
                    });
                    self.monitors.borrow_mut().push(monitor);
                }
                Err(e) => eprintln!("[theme] Cannot watch {}: {}", dir.display(), e),
            }
        }
    }

    /// Editors often write a file in several steps; coalesce into one reload.
    fn schedule_reload(self: &Rc<Self>) {
        if let Some(id) = self.pending_reload.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(Duration::from_millis(120), move || {
            if let Some(manager) = weak.upgrade() {
                manager.pending_reload.borrow_mut().take();
                manager.reload();
            }
        });
        *self.pending_reload.borrow_mut() = Some(id);
    }
}

/// `src` with its `base` key set to `id`; comments and layout are preserved.
fn with_base(src: &str, id: &str) -> Result<String, String> {
    let mut doc: toml_edit::DocumentMut = src.parse().map_err(|e| format!("{}", e))?;
    doc["base"] = toml_edit::value(id);
    Ok(doc.to_string())
}

fn preset_name(src: &str) -> Option<String> {
    ThemeFile::parse(src).ok()?.name
}

/// Maps the pre-token theme names stored in `config.toml` to preset ids.
fn legacy_preset_id(name: &str) -> &'static str {
    match name {
        "Rosé Pine" => "rose-pine",
        "Tokyo Soft" => "tokyo-soft",
        "Nord" => "nord",
        "Gruvbox" => "gruvbox",
        "Cozy Latte" => "cozy-latte",
        "Deep Dark" => "deep-dark",
        "High Contrast" => "high-contrast",
        _ => DEFAULT_PRESET,
    }
}

/// Writes a commented starter `theme.toml` on first run.
fn seed_theme_file(path: &Path, legacy_name: &str) -> std::io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, starter_theme(legacy_preset_id(legacy_name)))
}

fn starter_theme(base: &str) -> String {
    format!(
        r##"# Diptych theme — changes apply as soon as you save.
#
# `base` picks a preset: {presets}
# or any file in ~/.config/diptych/themes/<id>.toml.
base = "{base}"

# Override any token of the base theme below. Colors accept #rrggbb,
# #rrggbbaa, rgb(), rgba() or a reference such as "@accent".
# For anything else, write plain GTK CSS in ~/.config/diptych/user.css.

# dark = true

# [colors]
# window = "#1e1e2e"
# sidebar = "#181825"
# surface = "#313244"
# accent = "#cba6f7"
# accent-text = "@window"
# danger = "#f38ba8"

# [files]              # per file-kind colors
# folder = "@accent"
# image = "#a6e3a1"

# [shape]
# radius = 14                # corner radius in px (0 = square)
# density = "comfortable"    # compact | comfortable | spacious

# [fonts]
# ui = "Inter, Cantarell, sans-serif"
# mono = "JetBrains Mono, monospace"
# scale = 1.0
"##,
        presets = BUILTIN_PRESETS
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
            .join(", "),
        base = base,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_theme_parses_and_uses_legacy_choice() {
        let src = starter_theme(legacy_preset_id("Nord"));
        let file = ThemeFile::parse(&src).unwrap();
        assert_eq!(file.base.as_deref(), Some("nord"));
        assert_eq!(legacy_preset_id("Catppuccin"), "catppuccin-mocha");
        assert_eq!(legacy_preset_id("something else"), "catppuccin-mocha");
    }

    #[test]
    fn set_base_keeps_comments_and_overrides() {
        let src = "# my comment\nbase = \"nord\"\n\n[colors]\naccent = \"#ff0000\" # red!\n";
        let out = with_base(src, "gruvbox").unwrap();
        assert!(out.contains("# my comment"));
        assert!(out.contains("base = \"gruvbox\""));
        assert!(out.contains("accent = \"#ff0000\" # red!"));
        // A file without `base` (e.g. only overrides) gets one added.
        assert!(with_base("[colors]\naccent = \"#fff\"\n", "nord")
            .unwrap()
            .contains("base = \"nord\""));
    }
}
