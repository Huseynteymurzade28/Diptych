use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════
//  Theme Enum
// ═══════════════════════════════════════════════

/// Every selectable theme in the application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Catppuccin,
    RosePine,
    TokyoSoft,
    Nord,
    Gruvbox,
    CozyLatte,
    DeepDark,
    HighContrast,
}

impl Theme {
    /// Ordered list of all themes (used by settings dropdown).
    pub fn all() -> Vec<Theme> {
        vec![
            Theme::Catppuccin,
            Theme::RosePine,
            Theme::TokyoSoft,
            Theme::Nord,
            Theme::Gruvbox,
            Theme::CozyLatte,
            Theme::DeepDark,
            Theme::HighContrast,
        ]
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Catppuccin => "Catppuccin",
            Theme::RosePine => "Rosé Pine",
            Theme::TokyoSoft => "Tokyo Soft",
            Theme::Nord => "Nord",
            Theme::Gruvbox => "Gruvbox",
            Theme::CozyLatte => "Cozy Latte",
            Theme::DeepDark => "Deep Dark",
            Theme::HighContrast => "High Contrast",
        }
    }

    /// Returns all theme display names as string slices (for GTK StringList).
    pub fn all_names() -> Vec<&'static str> {
        Theme::all().iter().map(|t| t.display_name()).collect()
    }

    /// Look up a `Theme` from its display name string.
    pub fn from_name(name: &str) -> Theme {
        Theme::all()
            .into_iter()
            .find(|t| t.display_name() == name)
            .unwrap_or(Theme::Catppuccin)
    }

    /// Build the `ColorPalette` for this theme.
    pub fn palette(&self) -> ColorPalette {
        match self {
            Theme::Catppuccin => ColorPalette {
                // Catppuccin Mocha – cozy purple-blue dark palette
                bg_base: "#1e1e2e",
                bg_surface: "#181825",
                bg_overlay: "#313244",
                bg_hover: "#45475a",
                fg_primary: "#cdd6f4",
                fg_secondary: "#a6adc8",
                fg_muted: "#6c7086",
                fg_subtle: "#bac2de",
                accent: "#89b4fa",
                accent_hover: "#b4d0fb",
                border: "rgba(205, 214, 244, 0.06)",
                border_hover: "rgba(137, 180, 250, 0.25)",
                shadow: "rgba(0, 0, 0, 0.18)",
                shadow_hover: "rgba(0, 0, 0, 0.28)",
                accent_shadow: "rgba(137, 180, 250, 0.2)",
            },
            Theme::RosePine => ColorPalette {
                bg_base: "#191724",
                bg_surface: "#1f1d2e",
                bg_overlay: "#26233a",
                bg_hover: "#2a283e",
                fg_primary: "#e0def4",
                fg_secondary: "#908caa",
                fg_muted: "#6e6a86",
                fg_subtle: "#e0def4",
                accent: "#c4a7e7",
                accent_hover: "#d4bff0",
                border: "rgba(224, 222, 244, 0.06)",
                border_hover: "rgba(196, 167, 231, 0.25)",
                shadow: "rgba(0, 0, 0, 0.18)",
                shadow_hover: "rgba(0, 0, 0, 0.28)",
                accent_shadow: "rgba(196, 167, 231, 0.2)",
            },
            Theme::TokyoSoft => ColorPalette {
                bg_base: "#1a1b26",
                bg_surface: "#16161e",
                bg_overlay: "#292e42",
                bg_hover: "#3b4261",
                fg_primary: "#c0caf5",
                fg_secondary: "#565f89",
                fg_muted: "#565f89",
                fg_subtle: "#c0caf5",
                accent: "#7aa2f7",
                accent_hover: "#9bb8f9",
                border: "rgba(192, 202, 245, 0.06)",
                border_hover: "rgba(122, 162, 247, 0.25)",
                shadow: "rgba(0, 0, 0, 0.18)",
                shadow_hover: "rgba(0, 0, 0, 0.28)",
                accent_shadow: "rgba(122, 162, 247, 0.2)",
            },
            Theme::Nord => ColorPalette {
                bg_base: "#2e3440",
                bg_surface: "#242933",
                bg_overlay: "#3b4252",
                bg_hover: "#434c5e",
                fg_primary: "#d8dee9",
                fg_secondary: "#4c566a",
                fg_muted: "#4c566a",
                fg_subtle: "#d8dee9",
                accent: "#88c0d0",
                accent_hover: "#a3d1de",
                border: "rgba(216, 222, 233, 0.06)",
                border_hover: "rgba(136, 192, 208, 0.25)",
                shadow: "rgba(0, 0, 0, 0.18)",
                shadow_hover: "rgba(0, 0, 0, 0.28)",
                accent_shadow: "rgba(136, 192, 208, 0.2)",
            },
            Theme::Gruvbox => ColorPalette {
                bg_base: "#282828",
                bg_surface: "#1d2021",
                bg_overlay: "#3c3836",
                bg_hover: "#504945",
                fg_primary: "#ebdbb2",
                fg_secondary: "#928374",
                fg_muted: "#928374",
                fg_subtle: "#ebdbb2",
                accent: "#d79921",
                accent_hover: "#e5b84a",
                border: "rgba(235, 219, 178, 0.06)",
                border_hover: "rgba(215, 153, 33, 0.25)",
                shadow: "rgba(0, 0, 0, 0.18)",
                shadow_hover: "rgba(0, 0, 0, 0.28)",
                accent_shadow: "rgba(215, 153, 33, 0.2)",
            },
            Theme::CozyLatte => ColorPalette {
                // Warm pastel light theme – cozy coffeehouse vibe
                bg_base: "#eff1f5",
                bg_surface: "#e6e9ef",
                bg_overlay: "#dce0e8",
                bg_hover: "#ccd0da",
                fg_primary: "#4c4f69",
                fg_secondary: "#6c6f85",
                fg_muted: "#8c8fa1",
                fg_subtle: "#5c5f77",
                accent: "#dc8a78",
                accent_hover: "#e6a192",
                border: "rgba(76, 79, 105, 0.10)",
                border_hover: "rgba(220, 138, 120, 0.30)",
                shadow: "rgba(0, 0, 0, 0.06)",
                shadow_hover: "rgba(0, 0, 0, 0.12)",
                accent_shadow: "rgba(220, 138, 120, 0.15)",
            },
            Theme::DeepDark => ColorPalette {
                // True black AMOLED-style dark with vibrant accents
                bg_base: "#0a0a0f",
                bg_surface: "#111118",
                bg_overlay: "#1a1a24",
                bg_hover: "#252530",
                fg_primary: "#e8e8ef",
                fg_secondary: "#8888a0",
                fg_muted: "#555566",
                fg_subtle: "#ccccdd",
                accent: "#7c6ff0",
                accent_hover: "#9d93f5",
                border: "rgba(232, 232, 239, 0.06)",
                border_hover: "rgba(124, 111, 240, 0.30)",
                shadow: "rgba(0, 0, 0, 0.40)",
                shadow_hover: "rgba(0, 0, 0, 0.55)",
                accent_shadow: "rgba(124, 111, 240, 0.25)",
            },
            Theme::HighContrast => ColorPalette {
                // Accessibility-first – maximum contrast, clear outlines
                bg_base: "#000000",
                bg_surface: "#0a0a0a",
                bg_overlay: "#1a1a1a",
                bg_hover: "#2a2a2a",
                fg_primary: "#ffffff",
                fg_secondary: "#cccccc",
                fg_muted: "#999999",
                fg_subtle: "#dddddd",
                accent: "#ffdd00",
                accent_hover: "#ffee55",
                border: "rgba(255, 255, 255, 0.20)",
                border_hover: "rgba(255, 221, 0, 0.50)",
                shadow: "rgba(0, 0, 0, 0.50)",
                shadow_hover: "rgba(0, 0, 0, 0.65)",
                accent_shadow: "rgba(255, 221, 0, 0.25)",
            },
        }
    }

    /// Generates the full GTK CSS string for this theme.
    pub fn to_css(&self) -> String {
        self.palette().generate_css()
    }
}

// ═══════════════════════════════════════════════
//  Color Palette
// ═══════════════════════════════════════════════

/// A complete color palette that drives every CSS rule.
/// No hard-coded colors outside of this struct.
#[derive(Debug, Clone)]
pub struct ColorPalette {
    // Backgrounds
    pub bg_base: &'static str,
    pub bg_surface: &'static str,
    pub bg_overlay: &'static str,
    pub bg_hover: &'static str,

    // Foregrounds
    pub fg_primary: &'static str,
    pub fg_secondary: &'static str,
    pub fg_muted: &'static str,
    pub fg_subtle: &'static str,

    // Accent
    pub accent: &'static str,
    pub accent_hover: &'static str,

    // Borders & Shadows
    pub border: &'static str,
    pub border_hover: &'static str,
    pub shadow: &'static str,
    pub shadow_hover: &'static str,
    pub accent_shadow: &'static str,
}

impl ColorPalette {
    /// Generates the full GTK4 CSS from palette colors.
    pub fn generate_css(&self) -> String {
        let p = self;
        format!(
            r#"
/* ── Base ── */
window {{
    background-color: {bg_base};
    color: {fg_primary};
    font-family: 'Inter', 'Cantarell', sans-serif;
}}

/* ── Sidebar ── */
.sidebar {{
    background-color: {bg_surface};
    border-right: 1px solid {border};
}}
.sidebar-title {{
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 1px;
    color: {fg_secondary};
    padding: 6px 14px 4px 14px;
}}
.place-btn {{
    border-radius: 10px;
    padding: 6px 10px;
    margin: 1px 4px;
    background: transparent;
    color: {fg_primary};
    border: none;
    font-size: 13px;
    transition: background 150ms ease;
}}
.place-btn:hover {{
    background-color: {bg_overlay};
}}
.place-btn:active, .place-btn:checked {{
    background-color: {bg_hover};
}}

/* ── Toolbar ── */
.toolbar {{
    background: transparent;
    padding: 4px 6px;
}}
.toolbar-btn {{
    border-radius: 10px;
    padding: 6px 8px;
    min-width: 28px;
    min-height: 28px;
    background: transparent;
    color: {fg_secondary};
    border: none;
    transition: background 150ms ease;
}}
.toolbar-btn:hover {{
    background-color: {bg_overlay};
    color: {fg_primary};
}}
.toolbar-btn:active, .toolbar-btn:checked {{
    background-color: {bg_hover};
    color: {accent};
}}

/* ── Header / Breadcrumb ── */
.header-bar {{
    background-color: {bg_surface};
    border-bottom: 1px solid {border};
    padding: 4px 12px;
    min-height: 38px;
}}
.breadcrumb-label {{
    font-size: 13px;
    font-weight: 600;
    color: {fg_secondary};
}}
.breadcrumb-label-active {{
    color: {fg_primary};
}}

/* ── File Cards (Grid Mode) ── */
.file-card {{
    background-color: {bg_overlay};
    border-radius: 14px;
    padding: 14px 10px 10px 10px;
    border: 1px solid {border};
    box-shadow: 0 2px 8px {shadow};
    transition: all 180ms ease;
    margin: 4px;
}}
.file-card:hover {{
    background-color: {bg_hover};
    border-color: {border_hover};
    box-shadow: 0 4px 16px {shadow_hover};
}}
.file-card-name {{
    font-size: 12px;
    font-weight: 500;
    color: {fg_primary};
}}
.file-card-meta {{
    font-size: 10px;
    color: {fg_muted};
    margin-top: 2px;
}}

/* ── File Row (List Mode) ── */
.file-row {{
    border-radius: 10px;
    padding: 4px 10px;
    margin: 1px 4px;
    background: transparent;
    color: {fg_primary};
    border: none;
    transition: background 150ms ease;
}}
.file-row:hover {{
    background-color: {bg_overlay};
}}
.file-row-meta {{
    font-size: 10px;
    color: {fg_muted};
}}

/* ── Group Header ── */
.group-header {{
    font-size: 12px;
    font-weight: 700;
    color: {accent};
    padding: 8px 12px 4px 12px;
    letter-spacing: 0.5px;
}}

/* ── Inspector Panel ── */
.inspector {{
    background-color: {bg_base};
}}
.inspector-title {{
    font-size: 18px;
    font-weight: 700;
    color: {fg_primary};
}}
.inspector-subtitle {{
    font-size: 12px;
    color: {fg_muted};
}}
.inspector-meta-label {{
    font-size: 11px;
    color: {fg_secondary};
    font-weight: 600;
}}
.inspector-meta-value {{
    font-size: 11px;
    color: {fg_subtle};
}}

/* ── Buttons ── */
.btn-primary {{
    background-color: {accent};
    color: {bg_base};
    border-radius: 12px;
    padding: 8px 20px;
    font-weight: 600;
    border: none;
    box-shadow: 0 2px 8px {accent_shadow};
    transition: all 150ms ease;
}}
.btn-primary:hover {{
    background-color: {accent_hover};
    box-shadow: 0 4px 12px {accent_shadow};
}}
.btn-secondary {{
    background-color: {bg_overlay};
    color: {fg_primary};
    border-radius: 12px;
    padding: 8px 16px;
    font-weight: 600;
    border: 1px solid {border_hover};
    transition: all 150ms ease;
}}
.btn-secondary:hover {{
    background-color: {bg_hover};
    border-color: {accent};
    color: {fg_primary};
}}

/* ── Entries ── */
entry {{
    background-color: {bg_overlay};
    color: {fg_primary};
    border: 1px solid {border};
    border-radius: 10px;
    padding: 6px 10px;
    caret-color: {accent};
}}
entry:focus {{
    border-color: {border_hover};
}}

/* ── Popover ── */
popover {{
    background-color: {bg_base};
    color: {fg_primary};
    border: 1px solid {border};
    border-radius: 14px;
    box-shadow: 0 8px 32px {shadow_hover};
}}
popover contents {{
    border-radius: 14px;
    padding: 4px;
}}

/* ── Scrollbar ── */
scrollbar slider {{
    background-color: {bg_hover};
    border-radius: 99px;
    min-width: 6px;
    min-height: 6px;
}}
scrollbar slider:hover {{
    background-color: {fg_muted};
}}

/* ── Separators ── */
separator {{
    background-color: {border};
    min-height: 1px;
}}

/* ── Scale (Slider) ── */
scale trough {{
    background-color: {bg_overlay};
    border-radius: 99px;
    min-height: 6px;
}}
scale highlight {{
    background-color: {accent};
    border-radius: 99px;
}}
scale slider {{
    background-color: {fg_primary};
    border-radius: 50%;
    min-width: 16px;
    min-height: 16px;
    box-shadow: 0 1px 4px {shadow};
}}

/* ── Switch / Toggle ── */
switch {{
    background-color: {bg_overlay};
    border-radius: 99px;
}}
switch:checked {{
    background-color: {accent};
}}

/* ── Settings Panel ── */
.settings-section-title {{
    font-size: 13px;
    font-weight: 700;
    color: {accent};
    letter-spacing: 0.5px;
}}
.settings-label {{
    font-size: 12px;
    color: {fg_subtle};
}}
.settings-panel {{
    background-color: {bg_surface};
    border-radius: 16px;
    padding: 16px;
}}

/* ── Context Menu ── */
.context-menu {{
    background-color: {bg_base};
    border: 1px solid {border};
    border-radius: 14px;
}}
.context-menu-item {{
    border-radius: 8px;
    padding: 6px 12px;
    background: transparent;
    color: {fg_primary};
    border: none;
    font-size: 13px;
    transition: background 150ms ease;
}}
.context-menu-item:hover {{
    background-color: {bg_overlay};
}}

/* ── Creation Popover Buttons ── */
.creation-btn {{
    font-weight: 700;
    font-size: 13px;
    min-width: 60px;
    padding: 8px 18px;
    letter-spacing: 0.3px;
}}

.context-menu-danger {{
    color: #f38ba8;
}}
.context-menu-danger:hover {{
    background-color: rgba(243, 139, 168, 0.15);
}}
.context-menu-title {{
    font-size: 12px;
    font-weight: 700;
    color: {accent};
    margin-bottom: 4px;
}}

/* ── Hamburger Menu ── */
.hamburger-title {{
    font-size: 14px;
    font-weight: 700;
    color: {accent};
    letter-spacing: 0.5px;
}}

/* ── Graph View ── */
.graph-view {{
    background-color: {bg_base};
}}

/* ── Media Preview ── */
.preview-container {{
    padding: 16px;
    background-color: {bg_overlay};
    border-radius: 14px;
    border: 1px solid {border};
}}
.preview-image {{
    border-radius: 10px;
}}
.preview-loading-label {{
    font-size: 12px;
    color: {fg_muted};
}}
.preview-dimension-label {{
    font-size: 10px;
    color: {fg_muted};
    margin-top: 4px;
}}
.preview-error-label {{
    font-size: 12px;
    color: #f38ba8;
}}
.preview-video-placeholder {{
    padding: 24px;
    background-color: {bg_overlay};
    border-radius: 14px;
    border: 1px solid {border};
}}
.preview-play-icon {{
    color: {accent};
    opacity: 0.8;
}}
.preview-video-name {{
    font-size: 13px;
    font-weight: 600;
    color: {fg_primary};
}}
.preview-hint-label {{
    font-size: 10px;
    color: {fg_muted};
}}
.preview-tooltip-image {{
    border-radius: 6px;
}}

/* ── Icon Color Classes (Colorful theme) ── */
.icon-folder {{ color: {accent}; }}
.icon-rust {{ color: #fab387; }}
.icon-python {{ color: #89b4fa; }}
.icon-js {{ color: #f9e2af; }}
.icon-c {{ color: #74c7ec; }}
.icon-java {{ color: #f38ba8; }}
.icon-go {{ color: #a6e3a1; }}
.icon-script {{ color: #cba6f7; }}
.icon-image {{ color: #a6e3a1; }}
.icon-audio {{ color: #cba6f7; }}
.icon-video {{ color: #f38ba8; }}
.icon-archive {{ color: #f9e2af; }}
.icon-pdf {{ color: #eba0ac; }}
.icon-web {{ color: #94e2d5; }}
.icon-text {{ color: {fg_muted}; }}
.icon-config {{ color: #94e2d5; }}
.icon-default {{ color: {fg_secondary}; }}

/* ── Tree View ── */
.tree-view-container {{
    padding: 6px 8px;
}}

.tree-row-btn {{
    border-radius: 10px;
    padding: 4px 12px 4px 6px;
    margin: 1px 4px;
    min-height: 38px;
    background: transparent;
    color: {fg_primary};
    border: 1px solid transparent;
    transition: background 120ms ease, border-color 120ms ease, box-shadow 120ms ease;
}}
.tree-row-btn:hover {{
    background-color: {bg_overlay};
    box-shadow: 0 1px 4px {shadow};
}}
.tree-row-selected {{
    background-color: rgba(136, 192, 208, 0.14);
    border: 1px solid {border_hover};
    box-shadow: inset 3px 0 0 {accent};
}}
.tree-row-selected:hover {{
    background-color: rgba(136, 192, 208, 0.20);
}}

/* Guide lines — subtle vertical bars for nesting */
.tree-guide-line {{
    border-left: 1px solid {border};
    margin-left: 10px;
    min-width: 1px;
}}
.tree-guide-spacer {{
    min-width: 20px;
}}

/* Disclosure arrow */
.tree-arrow {{
    min-width: 26px;
    min-height: 26px;
    padding: 0px;
    margin: 0px;
    border-radius: 7px;
    font-size: 13px;
    color: {fg_muted};
    background: transparent;
    border: none;
    transition: color 120ms ease, background 120ms ease;
}}
.tree-arrow:hover {{
    color: {accent};
    background-color: {bg_hover};
}}
.tree-arrow-open {{
    color: {accent};
}}

/* File dot (leaf node indicator) */
.tree-file-dot {{
    font-size: 9px;
    color: {fg_muted};
    opacity: 0.30;
    min-width: 26px;
}}

/* Icon styling — bigger for scannability */
.tree-icon {{
    min-width: 22px;
    min-height: 22px;
    -gtk-icon-style: symbolic;
}}
/* Colorful theme: let GTK render the icon with its native colors */
.tree-icon-colorful {{
    -gtk-icon-style: regular;
}}
.tree-icon-folder {{
    color: {accent};
}}

/* Force icon recoloring so .icon-* color classes work on symbolic icons */
.icon-folder image, .tree-icon.icon-folder {{ color: {accent}; }}
.icon-rust image, .tree-icon.icon-rust {{ color: #fab387; }}
.icon-python image, .tree-icon.icon-python {{ color: #89b4fa; }}
.icon-js image, .tree-icon.icon-js {{ color: #f9e2af; }}
.icon-c image, .tree-icon.icon-c {{ color: #74c7ec; }}
.icon-java image, .tree-icon.icon-java {{ color: #f38ba8; }}
.icon-go image, .tree-icon.icon-go {{ color: #a6e3a1; }}
.icon-script image, .tree-icon.icon-script {{ color: #cba6f7; }}
.icon-image image, .tree-icon.icon-image {{ color: #a6e3a1; }}
.icon-audio image, .tree-icon.icon-audio {{ color: #cba6f7; }}
.icon-video image, .tree-icon.icon-video {{ color: #f38ba8; }}
.icon-archive image, .tree-icon.icon-archive {{ color: #f9e2af; }}
.icon-pdf image, .tree-icon.icon-pdf {{ color: #eba0ac; }}
.icon-web image, .tree-icon.icon-web {{ color: #94e2d5; }}
.icon-text image, .tree-icon.icon-text {{ color: {fg_muted}; }}
.icon-config image, .tree-icon.icon-config {{ color: #94e2d5; }}
.icon-default image, .tree-icon.icon-default {{ color: {fg_secondary}; }}

/* Name labels — bigger, more readable */
.tree-name {{
    font-size: 14px;
    letter-spacing: 0.2px;
}}
.tree-name-dir {{
    font-weight: 700;
    color: {fg_primary};
}}
.tree-name-file {{
    font-weight: 400;
    color: {fg_subtle};
}}

/* ── File Extension Colors (Nord / Tokyo Night) ── */
.tree-ext-rs  {{ color: #d08770; }}   /* Rust — warm orange */
.tree-ext-py  {{ color: #ebcb8b; }}   /* Python — golden yellow */
.tree-ext-js  {{ color: #ebcb8b; }}   /* JavaScript — yellow */
.tree-ext-ts  {{ color: #81a1c1; }}   /* TypeScript — steel blue */
.tree-ext-jsx {{ color: #88c0d0; }}   /* JSX — cyan */
.tree-ext-tsx {{ color: #81a1c1; }}   /* TSX — blue */
.tree-ext-c   {{ color: #81a1c1; }}   /* C — blue */
.tree-ext-cpp {{ color: #81a1c1; }}   /* C++ — blue */
.tree-ext-h   {{ color: #8fbcbb; }}   /* Header — teal */
.tree-ext-go  {{ color: #a3be8c; }}   /* Go — sage green */
.tree-ext-java {{ color: #bf616a; }}  /* Java — aurora red */
.tree-ext-kt  {{ color: #b48ead; }}   /* Kotlin — purple */
.tree-ext-rb  {{ color: #bf616a; }}   /* Ruby — red */
.tree-ext-swift {{ color: #d08770; }} /* Swift — orange */
.tree-ext-cs  {{ color: #b48ead; }}   /* C# — purple */
.tree-ext-lua {{ color: #81a1c1; }}   /* Lua — blue */
.tree-ext-sh  {{ color: #a3be8c; }}   /* Shell — green */
.tree-ext-fish {{ color: #a3be8c; }}
.tree-ext-zsh {{ color: #a3be8c; }}
.tree-ext-bash {{ color: #a3be8c; }}
.tree-ext-html {{ color: #bf616a; }}  /* HTML — red */
.tree-ext-htm {{ color: #bf616a; }}
.tree-ext-css {{ color: #88c0d0; }}   /* CSS — frost cyan */
.tree-ext-scss {{ color: #b48ead; }}  /* SCSS — purple */
.tree-ext-md  {{ color: #4c566a; }}   /* Markdown — muted gray */
.tree-ext-txt {{ color: {fg_muted}; }}
.tree-ext-log {{ color: {fg_muted}; }}
.tree-ext-csv {{ color: {fg_muted}; }}
.tree-ext-json {{ color: #ebcb8b; }}  /* JSON — yellow */
.tree-ext-toml {{ color: #d08770; }}  /* TOML — orange */
.tree-ext-yaml {{ color: #a3be8c; }}  /* YAML — green */
.tree-ext-yml {{ color: #a3be8c; }}
.tree-ext-xml {{ color: #81a1c1; }}   /* XML — blue */
.tree-ext-png {{ color: #a3be8c; }}   /* Images — green */
.tree-ext-jpg {{ color: #a3be8c; }}
.tree-ext-jpeg {{ color: #a3be8c; }}
.tree-ext-gif {{ color: #a3be8c; }}
.tree-ext-svg {{ color: #ebcb8b; }}   /* SVG — yellow */
.tree-ext-webp {{ color: #a3be8c; }}
.tree-ext-mp3 {{ color: #b48ead; }}   /* Audio — purple */
.tree-ext-flac {{ color: #b48ead; }}
.tree-ext-ogg {{ color: #b48ead; }}
.tree-ext-wav {{ color: #b48ead; }}
.tree-ext-mp4 {{ color: #bf616a; }}   /* Video — red */
.tree-ext-mkv {{ color: #bf616a; }}
.tree-ext-avi {{ color: #bf616a; }}
.tree-ext-mov {{ color: #bf616a; }}
.tree-ext-webm {{ color: #bf616a; }}
.tree-ext-zip {{ color: #d08770; }}   /* Archives — orange */
.tree-ext-tar {{ color: #d08770; }}
.tree-ext-gz  {{ color: #d08770; }}
.tree-ext-7z  {{ color: #d08770; }}
.tree-ext-rar {{ color: #d08770; }}
.tree-ext-pdf {{ color: #bf616a; }}   /* PDF — red */
.tree-ext-nix {{ color: #81a1c1; }}   /* Nix — blue */
.tree-ext-lock {{ color: {fg_muted}; }} /* Lock files — muted */
.tree-ext-none {{ color: {fg_secondary}; }} /* No extension */

/* Child count badge for directories */
.tree-badge {{
    font-size: 9px;
    font-weight: 700;
    color: {fg_muted};
    background-color: {bg_overlay};
    border-radius: 99px;
    padding: 1px 7px;
    margin-right: 2px;
    min-height: 16px;
}}

/* File size metadata */
.tree-meta {{
    font-size: 10px;
    color: {fg_muted};
    margin-right: 4px;
}}

/* ── Empty Directory State ── */
.tree-empty-container {{
    padding: 6px 14px;
    margin: 2px 4px;
    border-radius: 10px;
    background-color: {bg_overlay};
    border: 1px solid {border};
}}
.tree-empty-icon {{
    opacity: 0.70;
    color: {accent};
}}
.tree-empty-hint {{
    font-size: 12.5px;
    font-style: italic;
    font-weight: 500;
    color: {fg_secondary};
    opacity: 1.0;
    letter-spacing: 0.3px;
}}

/* ── Drag Ghost Card (floating preview during drag) ── */
.drag-ghost-card {{
    background-color: {bg_surface};
    border: 1px solid {border_hover};
    border-radius: 12px;
    padding: 8px 16px 8px 12px;
    box-shadow: 0 8px 24px {shadow_hover}, 0 2px 6px {shadow};
    min-height: 36px;
}}
.drag-ghost-icon-file {{
    opacity: 0.9;
    color: {fg_subtle};
}}
.drag-ghost-icon-folder {{
    color: {accent};
}}
.drag-ghost-name {{
    font-size: 13px;
    font-weight: 600;
    color: {fg_primary};
    letter-spacing: 0.2px;
}}
.drag-ghost-badge {{
    font-size: 10px;
    font-weight: 600;
    color: {fg_muted};
    background-color: {bg_overlay};
    border-radius: 99px;
    padding: 2px 8px;
    margin-left: 4px;
}}
"#,
            bg_base = p.bg_base,
            bg_surface = p.bg_surface,
            bg_overlay = p.bg_overlay,
            bg_hover = p.bg_hover,
            fg_primary = p.fg_primary,
            fg_secondary = p.fg_secondary,
            fg_muted = p.fg_muted,
            fg_subtle = p.fg_subtle,
            accent = p.accent,
            accent_hover = p.accent_hover,
            border = p.border,
            border_hover = p.border_hover,
            shadow = p.shadow,
            shadow_hover = p.shadow_hover,
            accent_shadow = p.accent_shadow,
        )
    }
}
