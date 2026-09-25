// ─── Theme Module ───
// Token-based theming: presets + `theme.toml` → CSS, `user.css` on top,
// both hot-reloaded from `~/.config/diptych/`.

mod color;
mod css;
mod model;

pub use model::{Mode, Theme, ThemeFile};

use gtk4::CssProvider;
use model::{Presets, BUILTIN_PRESETS, DEFAULT_PRESET};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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
    watches: RefCell<Vec<crate::config::watch::Watch>>,
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
            watches: RefCell::new(vec![]),
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

        // `mode = "system"`: re-pick the preset when the desktop flips
        // between light and dark.
        let weak = Rc::downgrade(&manager);
        adw::StyleManager::default().connect_dark_notify(move |_| {
            if let Some(m) = weak.upgrade() {
                m.reload();
            }
        });
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
        let style = adw::StyleManager::default();
        match self.load_theme(style.is_dark()) {
            Ok((theme, mode)) => match css::generate(&theme) {
                Ok(css) => {
                    self.theme_provider.load_from_data(&css);
                    let scheme = match mode {
                        Some(Mode::System) => adw::ColorScheme::Default,
                        _ if theme.dark => adw::ColorScheme::ForceDark,
                        _ => adw::ColorScheme::ForceLight,
                    };
                    // Setting the same scheme again is a no-op, so this
                    // can't loop through `connect_dark_notify`.
                    if style.color_scheme() != scheme {
                        style.set_color_scheme(scheme);
                    }
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

    fn load_theme(&self, system_dark: bool) -> Result<(Theme, Option<Mode>), String> {
        let src = std::fs::read_to_string(self.theme_path()).unwrap_or_default();
        let mut file = ThemeFile::parse(&src)?;
        let mode = file.mode();
        file.base = file.base_for(system_dark);
        let user_dir = self.dir.join(THEMES_DIR);
        let theme = Theme::resolve(
            &file,
            &Presets {
                user_dir: Some(&user_dir),
            },
        )?;
        Ok((theme, mode))
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
        let reload = |weak: std::rc::Weak<Self>| {
            move || {
                if let Some(manager) = weak.upgrade() {
                    manager.reload();
                }
            }
        };
        let own_files = crate::config::watch::watch(
            std::slice::from_ref(&self.dir),
            |name| name == Path::new(THEME_FILE) || name == Path::new(USER_CSS),
            reload(Rc::downgrade(self)),
        );
        let presets = crate::config::watch::watch(
            &[self.dir.join(THEMES_DIR)],
            |name| name.extension().is_some_and(|e| e == "toml"),
            reload(Rc::downgrade(self)),
        );
        self.watches.borrow_mut().extend([own_files, presets]);
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

# Follow the desktop's light/dark preference (GNOME, KDE, Hyprland via
# the settings portal): uncomment to use a light preset in light mode.
# base-light = "cozy-latte"
# mode = "system"          # system | dark | light

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
# ui = "system"             # desktop font, or e.g. "Inter, sans-serif"
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
