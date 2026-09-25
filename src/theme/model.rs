use super::color::Rgba;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

// ═══════════════════════════════════════════════
//  Theme Files
// ═══════════════════════════════════════════════
//
// A theme file (built-in preset, `~/.config/diptych/themes/*.toml`, or the
// user's `~/.config/diptych/theme.toml`) may set any subset of tokens.
// Resolution: default preset ← `base` (recursively) ← this file.

/// Color tokens every theme ends up with. Used by `base.css`.
pub const COLOR_TOKENS: &[&str] = &[
    "window",
    "sidebar",
    "surface",
    "hover",
    "text",
    "text-secondary",
    "text-muted",
    "text-subtle",
    "accent",
    "accent-hover",
    "accent-text",
    "border",
    "border-hover",
    "shadow",
    "shadow-hover",
    "danger",
];

/// Per-file-kind colors, `[files]` in TOML, `file-<kind>` in `base.css`.
pub const FILE_TOKENS: &[&str] = &[
    "folder", "rust", "python", "js", "c", "java", "go", "script", "image", "audio", "video",
    "archive", "pdf", "web", "text", "config", "default",
];

pub const DEFAULT_PRESET: &str = "catppuccin-mocha";

/// Built-in presets, embedded at compile time: (id, TOML source).
pub const BUILTIN_PRESETS: &[(&str, &str)] = &[
    (
        "catppuccin-mocha",
        include_str!("presets/catppuccin-mocha.toml"),
    ),
    ("rose-pine", include_str!("presets/rose-pine.toml")),
    ("tokyo-soft", include_str!("presets/tokyo-soft.toml")),
    ("nord", include_str!("presets/nord.toml")),
    ("gruvbox", include_str!("presets/gruvbox.toml")),
    ("cozy-latte", include_str!("presets/cozy-latte.toml")),
    ("deep-dark", include_str!("presets/deep-dark.toml")),
    ("high-contrast", include_str!("presets/high-contrast.toml")),
];

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    Compact,
    Comfortable,
    Spacious,
}

impl Density {
    /// Multiplier applied to paddings, margins and row heights.
    pub fn factor(self) -> f64 {
        match self {
            Density::Compact => 0.75,
            Density::Comfortable => 1.0,
            Density::Spacious => 1.25,
        }
    }
}

/// How the light/dark variant is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Follow the desktop's dark-style preference: `base` when dark,
    /// `base-light` when light (GNOME, KDE and others via the portal).
    System,
    Dark,
    Light,
}

/// Raw contents of one theme file. Every field is optional.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct ThemeFile {
    pub base: Option<String>,
    /// Preset used when the desktop prefers a light style (`mode = "system"`).
    pub base_light: Option<String>,
    pub mode: Option<Mode>,
    pub name: Option<String>,
    pub dark: Option<bool>,
    pub colors: BTreeMap<String, String>,
    pub files: BTreeMap<String, String>,
    pub shape: ShapeFile,
    pub fonts: FontsFile,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShapeFile {
    pub radius: Option<f64>,
    pub density: Option<Density>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FontsFile {
    pub ui: Option<String>,
    pub mono: Option<String>,
    pub scale: Option<f64>,
}

impl ThemeFile {
    /// Effective mode: explicit `mode`, else follow the system when a
    /// light preset is configured, else fixed to the preset's own style.
    pub fn mode(&self) -> Option<Mode> {
        self.mode.or(self.base_light.as_ref().map(|_| Mode::System))
    }

    /// The preset to use given the desktop's current preference.
    pub fn base_for(&self, system_dark: bool) -> Option<String> {
        let light = || self.base_light.clone().or_else(|| self.base.clone());
        match self.mode() {
            Some(Mode::Light) => light(),
            Some(Mode::System) if !system_dark => light(),
            _ => self.base.clone(),
        }
    }

    pub fn parse(src: &str) -> Result<ThemeFile, String> {
        let file: ThemeFile = toml::from_str(src).map_err(|e| e.to_string())?;
        file.validate()?;
        Ok(file)
    }

    fn validate(&self) -> Result<(), String> {
        for key in self.colors.keys() {
            if !COLOR_TOKENS.contains(&key.as_str()) {
                return Err(unknown("color", key, COLOR_TOKENS));
            }
        }
        for key in self.files.keys() {
            if !FILE_TOKENS.contains(&key.as_str()) {
                return Err(unknown("file color", key, FILE_TOKENS));
            }
        }
        if let Some(r) = self.shape.radius {
            if !(0.0..=64.0).contains(&r) {
                return Err(format!("shape.radius must be between 0 and 64, got {}", r));
            }
        }
        if let Some(s) = self.fonts.scale {
            if !(0.5..=2.0).contains(&s) {
                return Err(format!(
                    "fonts.scale must be between 0.5 and 2.0, got {}",
                    s
                ));
            }
        }
        for font in [&self.fonts.ui, &self.fonts.mono].into_iter().flatten() {
            // Values are pasted into CSS: keep them from closing the rule.
            if font.contains(['{', '}', ';']) {
                return Err(format!("font “{}” must not contain {{, }} or ;", font));
            }
        }
        Ok(())
    }
}

fn unknown(kind: &str, key: &str, known: &[&str]) -> String {
    format!(
        "unknown {} token “{}” (known: {})",
        kind,
        key,
        known.join(", ")
    )
}

// ═══════════════════════════════════════════════
//  Resolved Theme
// ═══════════════════════════════════════════════

/// A fully-resolved theme: every token has a concrete value.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub dark: bool,
    /// Keys: `COLOR_TOKENS` plus `file-<kind>` for `FILE_TOKENS`.
    pub colors: BTreeMap<String, Rgba>,
    pub radius: f64,
    pub density: Density,
    /// `None` = the desktop's font.
    pub font_ui: Option<String>,
    pub font_mono: String,
    pub font_scale: f64,
}

/// Where to look up `base = "<id>"` references.
pub trait PresetSource {
    /// Returns the TOML source of preset `id`, if known.
    fn preset(&self, id: &str) -> Option<String>;
}

/// Built-in presets, plus `*.toml` files in a user themes directory.
pub struct Presets<'a> {
    pub user_dir: Option<&'a Path>,
}

impl PresetSource for Presets<'_> {
    fn preset(&self, id: &str) -> Option<String> {
        // Plain ids only — `base = "../../etc/x"` must not read arbitrary files.
        let safe = !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !safe {
            return None;
        }
        if let Some(dir) = self.user_dir {
            if let Ok(src) = std::fs::read_to_string(dir.join(format!("{}.toml", id))) {
                return Some(src);
            }
        }
        BUILTIN_PRESETS
            .iter()
            .find(|(pid, _)| *pid == id)
            .map(|(_, src)| src.to_string())
    }
}

impl Theme {
    /// Resolves `file` against its `base` chain and the default preset.
    pub fn resolve(file: &ThemeFile, presets: &dyn PresetSource) -> Result<Theme, String> {
        let mut chain = vec![];
        let mut next = file.base.clone();
        while let Some(id) = next {
            if chain.len() >= 8 {
                return Err("theme `base` chain is too deep (or circular)".into());
            }
            let src = presets
                .preset(&id)
                .ok_or_else(|| format!("unknown base theme “{}”", id))?;
            let parent = ThemeFile::parse(&src).map_err(|e| format!("theme “{}”: {}", id, e))?;
            next = parent.base.clone();
            chain.push(parent);
        }
        // Root of every chain: the default preset fills any gaps.
        let default_src = presets
            .preset(DEFAULT_PRESET)
            .expect("default preset is built in");
        chain.push(ThemeFile::parse(&default_src)?);

        // Merge root → leaf, later files win.
        let mut raw: BTreeMap<String, String> = BTreeMap::new();
        let (mut name, mut dark) = (None, None);
        let (mut radius, mut density) = (None, None);
        let (mut ui, mut mono, mut scale) = (None, None, None);
        for f in chain.iter().rev().chain(std::iter::once(file)) {
            raw.extend(f.colors.iter().map(|(k, v)| (k.clone(), v.clone())));
            raw.extend(
                f.files
                    .iter()
                    .map(|(k, v)| (format!("file-{}", k), v.clone())),
            );
            name = f.name.clone().or(name);
            dark = f.dark.or(dark);
            radius = f.shape.radius.or(radius);
            density = f.shape.density.or(density);
            ui = f.fonts.ui.clone().or(ui);
            mono = f.fonts.mono.clone().or(mono);
            scale = f.fonts.scale.or(scale);
        }

        let mut colors = BTreeMap::new();
        for key in raw.keys() {
            colors.insert(key.clone(), resolve_color(&raw, key, 0)?);
        }
        let required = COLOR_TOKENS
            .iter()
            .map(|k| k.to_string())
            .chain(FILE_TOKENS.iter().map(|k| format!("file-{}", k)));
        for key in required {
            if !colors.contains_key(&key) {
                return Err(format!("theme is missing color “{}”", key));
            }
        }

        Ok(Theme {
            name: name.unwrap_or_else(|| "Custom".into()),
            dark: dark.unwrap_or(true),
            colors,
            radius: radius.unwrap_or(14.0),
            density: density.unwrap_or(Density::Comfortable),
            font_ui: ui.filter(|f: &String| !f.trim().eq_ignore_ascii_case("system")),
            font_mono: mono.unwrap_or_else(|| "monospace".into()),
            font_scale: scale.unwrap_or(1.0),
        })
    }

    pub fn color(&self, key: &str) -> Option<Rgba> {
        self.colors.get(key).copied()
    }
}

/// Resolves `@name` references (e.g. `accent-text = "@window"`).
fn resolve_color(raw: &BTreeMap<String, String>, key: &str, depth: u8) -> Result<Rgba, String> {
    if depth > 8 {
        return Err(format!("color reference loop at “{}”", key));
    }
    let value = raw
        .get(key)
        .ok_or_else(|| format!("reference to unknown color “@{}”", key))?;
    match value.trim().strip_prefix('@') {
        // `@accent` in [files] means the color token, `@file-rust` a file color.
        Some(target) => resolve_color(raw, target, depth + 1),
        None => Rgba::parse(value).map_err(|e| format!("{}: {}", key, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoUserThemes;
    impl PresetSource for NoUserThemes {
        fn preset(&self, id: &str) -> Option<String> {
            Presets { user_dir: None }.preset(id)
        }
    }

    fn resolve(src: &str) -> Result<Theme, String> {
        Theme::resolve(&ThemeFile::parse(src)?, &NoUserThemes)
    }

    #[test]
    fn every_builtin_preset_is_complete_on_its_own() {
        for (id, src) in BUILTIN_PRESETS {
            let file = ThemeFile::parse(src).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(file.base.is_none(), "{id} must not have a base");
            assert!(
                file.name.is_some() && file.dark.is_some(),
                "{id} needs name + dark"
            );
            for key in COLOR_TOKENS {
                assert!(
                    file.colors.contains_key(*key),
                    "{id} is missing color {key}"
                );
            }
            for key in FILE_TOKENS {
                assert!(
                    file.files.contains_key(*key),
                    "{id} is missing file color {key}"
                );
            }
            resolve(&format!("base = \"{id}\"")).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }

    #[test]
    fn empty_file_resolves_to_default_preset() {
        let t = resolve("").unwrap();
        assert_eq!(t.name, "Catppuccin Mocha");
        assert_eq!(t.color("accent").unwrap().to_string(), "#89b4fa");
    }

    #[test]
    fn overrides_and_references() {
        let t = resolve(
            r##"
            base = "nord"
            [colors]
            accent = "#ff0000"
            [files]
            folder = "@accent"
            rust = "@file-java"
            [shape]
            radius = 4
            density = "compact"
            "##,
        )
        .unwrap();
        assert_eq!(t.name, "Nord");
        assert_eq!(t.color("accent").unwrap().to_string(), "#ff0000");
        assert_eq!(t.color("file-folder").unwrap().to_string(), "#ff0000");
        assert_eq!(t.color("file-rust"), t.color("file-java"));
        // Untouched tokens come from Nord.
        assert_eq!(t.color("window").unwrap().to_string(), "#2e3440");
        assert_eq!(t.radius, 4.0);
        assert_eq!(t.density, Density::Compact);
    }

    #[test]
    fn helpful_errors() {
        let err = |src: &str| resolve(src).unwrap_err();
        assert!(err("base = \"nope\"").contains("unknown base theme"));
        assert!(err("[colors]\nacent = \"#fff\"").contains("unknown color token “acent”"));
        assert!(err("[colors]\naccent = \"@missing\"").contains("unknown color"));
        assert!(err("[colors]\naccent = \"@hover\"\nhover = \"@accent\"").contains("loop"));
        assert!(err("[colors]\naccent = \"blue\"").contains("unsupported color"));
        assert!(err("[shape]\nradius = 500").contains("radius"));
        assert!(err("[fonts]\nui = \"x; } window { color: red\"").contains("must not contain"));
        assert!(err("colour = 1").contains("unknown field"));
    }

    #[test]
    fn mode_picks_the_light_or_dark_base() {
        let f = ThemeFile::parse("base = \"nord\"\nbase-light = \"cozy-latte\"").unwrap();
        assert_eq!(
            f.mode(),
            Some(Mode::System),
            "base-light implies following the system"
        );
        assert_eq!(f.base_for(true).as_deref(), Some("nord"));
        assert_eq!(f.base_for(false).as_deref(), Some("cozy-latte"));

        let f = ThemeFile::parse("base = \"nord\"\nbase-light = \"cozy-latte\"\nmode = \"dark\"")
            .unwrap();
        assert_eq!(f.base_for(false).as_deref(), Some("nord"));

        let f = ThemeFile::parse("base = \"nord\"").unwrap();
        assert_eq!(
            f.mode(),
            None,
            "no base-light: fixed to the preset's own style"
        );
        assert_eq!(f.base_for(false).as_deref(), Some("nord"));
        assert!(ThemeFile::parse("mode = \"auto\"").is_err());
    }

    #[test]
    fn system_font_means_no_override() {
        assert_eq!(resolve("").unwrap().font_ui, None);
        let t = resolve("[fonts]\nui = \"Figtree\"").unwrap();
        assert_eq!(t.font_ui.as_deref(), Some("Figtree"));
    }

    #[test]
    fn base_ids_cannot_escape_the_themes_dir() {
        let presets = Presets {
            user_dir: Some(Path::new("/tmp")),
        };
        assert!(presets.preset("../etc/passwd").is_none());
        assert!(presets.preset("a/b").is_none());
        assert!(presets.preset("").is_none());
    }

    #[test]
    fn user_theme_dir_is_searched_first() {
        let dir = std::env::temp_dir().join(format!("diptych-theme-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("mine.toml"),
            "base = \"gruvbox\"\nname = \"Mine\"\n[colors]\naccent = \"#123456\"",
        )
        .unwrap();
        let presets = Presets {
            user_dir: Some(&dir),
        };
        let t = Theme::resolve(&ThemeFile::parse("base = \"mine\"").unwrap(), &presets).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(t.name, "Mine");
        assert_eq!(t.color("accent").unwrap().to_string(), "#123456");
        assert_eq!(t.color("window").unwrap().to_string(), "#282828");
    }
}
