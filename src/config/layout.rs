use serde::Deserialize;
use std::path::Path;

// ═══════════════════════════════════════════════
//  layout.toml — window chrome and panes
// ═══════════════════════════════════════════════
//
// Hot-reloaded like theme.toml. Every key is optional; see
// `starter_layout()` for the documented defaults.

pub const LAYOUT_FILE: &str = "layout.toml";

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct LayoutConfig {
    pub window: WindowLayout,
    pub sidebar: SidebarLayout,
    pub inspector: InspectorLayout,
    pub header: HeaderLayout,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct WindowLayout {
    pub decorations: Decorations,
    /// Below this width (px) the inspector turns into an overlay.
    pub collapse_inspector_below: u32,
    /// Below this width (px) the sidebar turns into an overlay too.
    pub collapse_sidebar_below: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decorations {
    /// Full header bar on floating desktops, no window buttons on tiling ones.
    Auto,
    /// Header bar with window buttons (layout from `gtk-decoration-layout`).
    Full,
    /// Header bar without window buttons.
    Minimal,
    /// No header bar at all.
    None,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct SidebarLayout {
    pub visible: bool,
    pub width: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct InspectorLayout {
    pub visible: bool,
    pub width: u32,
    pub position: Side,
    pub fields: Vec<InspectorField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InspectorField {
    Kind,
    Size,
    Modified,
    Created,
    Dimensions,
    Location,
    Permissions,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct HeaderLayout {
    pub start: Vec<HeaderItem>,
    pub center: Vec<HeaderItem>,
    pub end: Vec<HeaderItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeaderItem {
    SidebarToggle,
    Back,
    Forward,
    Up,
    Path,
    New,
    ViewSwitcher,
    InspectorToggle,
    Menu,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            decorations: Decorations::Auto,
            collapse_inspector_below: 900,
            collapse_sidebar_below: 600,
        }
    }
}

impl Default for SidebarLayout {
    fn default() -> Self {
        Self {
            visible: true,
            width: 220,
        }
    }
}

impl Default for InspectorLayout {
    fn default() -> Self {
        use InspectorField::*;
        Self {
            visible: true,
            width: 300,
            position: Side::Right,
            fields: vec![Kind, Size, Modified, Dimensions, Location, Permissions],
        }
    }
}

impl Default for HeaderLayout {
    fn default() -> Self {
        use HeaderItem::*;
        Self {
            start: vec![SidebarToggle, Back, Forward, Up],
            center: vec![Path],
            end: vec![New, ViewSwitcher, InspectorToggle, Menu],
        }
    }
}

impl LayoutConfig {
    pub fn parse(src: &str) -> Result<LayoutConfig, String> {
        let cfg: LayoutConfig = toml::from_str(src).map_err(|e| e.to_string())?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Reads `layout.toml` from `dir`; a missing file means defaults.
    pub fn load(dir: &Path) -> Result<LayoutConfig, String> {
        match std::fs::read_to_string(dir.join(LAYOUT_FILE)) {
            Ok(src) => Self::parse(&src),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn validate(&self) -> Result<(), String> {
        let check = |name: &str, v: u32, min: u32, max: u32| {
            if (min..=max).contains(&v) {
                Ok(())
            } else {
                Err(format!(
                    "{} must be between {} and {}, got {}",
                    name, min, max, v
                ))
            }
        };
        check("sidebar.width", self.sidebar.width, 120, 600)?;
        check("inspector.width", self.inspector.width, 180, 800)?;

        let h = &self.header;
        let all: Vec<HeaderItem> = h
            .start
            .iter()
            .chain(&h.center)
            .chain(&h.end)
            .copied()
            .collect();
        for (i, item) in all.iter().enumerate() {
            if all[..i].contains(item) {
                return Err(format!("header item {:?} is listed more than once", item));
            }
        }
        Ok(())
    }
}

/// Which decoration style `Auto` means on the current desktop.
/// `desktop` is `$XDG_CURRENT_DESKTOP` (colon-separated, e.g. "ubuntu:GNOME").
pub fn auto_decorations(desktop: &str) -> Decorations {
    // Tiling compositors manage windows themselves: close/maximize buttons
    // are dead weight there.
    const TILING: &[&str] = &[
        "hyprland", "sway", "niri", "river", "i3", "bspwm", "qtile", "dwl", "wayfire", "labwc",
    ];
    let tiling = desktop
        .split(':')
        .any(|d| TILING.contains(&d.trim().to_ascii_lowercase().as_str()));
    if tiling {
        Decorations::Minimal
    } else {
        Decorations::Full
    }
}

/// Writes a commented starter `layout.toml` on first run.
pub fn seed(dir: &Path) -> std::io::Result<()> {
    let path = dir.join(LAYOUT_FILE);
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(path, starter_layout())
}

pub fn starter_layout() -> &'static str {
    r#"# Diptych layout — changes apply as soon as you save.
# Every key is optional; the values below are the defaults.

[window]
# auto    = window buttons on GNOME/KDE, none on tiling compositors
#           (Hyprland, Sway, niri, river, …)
# full    = always show window buttons (order from your desktop settings)
# minimal = header bar without window buttons
# none    = no header bar at all (use the keyboard / F10 menu)
decorations = "auto"
collapse-inspector-below = 900   # px; narrower windows show it as an overlay
collapse-sidebar-below = 600

[sidebar]
visible = true
width = 220

[inspector]
visible = true
width = 300
position = "right"               # left | right
# kind, size, modified, created, dimensions, location, permissions
fields = ["kind", "size", "modified", "dimensions", "location", "permissions"]

[header]
# Items: sidebar-toggle, back, forward, up, path, new, view-switcher,
#        inspector-toggle, menu. Leave one out to hide it.
start = ["sidebar-toggle", "back", "forward", "up"]
center = ["path"]
end = ["new", "view-switcher", "inspector-toggle", "menu"]
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_file_equals_defaults() {
        assert_eq!(
            LayoutConfig::parse(starter_layout()).unwrap(),
            LayoutConfig::default()
        );
        assert_eq!(LayoutConfig::parse("").unwrap(), LayoutConfig::default());
    }

    #[test]
    fn partial_files_keep_other_defaults() {
        let cfg = LayoutConfig::parse("[inspector]\nposition = \"left\"\nwidth = 360").unwrap();
        assert_eq!(cfg.inspector.position, Side::Left);
        assert_eq!(cfg.inspector.width, 360);
        assert!(cfg.inspector.visible);
        assert_eq!(cfg.header, HeaderLayout::default());
    }

    #[test]
    fn rejects_mistakes() {
        let err = |s: &str| LayoutConfig::parse(s).unwrap_err();
        assert!(err("[sidebar]\nwidth = 5000").contains("sidebar.width"));
        assert!(err("[header]\nstart = [\"back\"]\nend = [\"back\"]").contains("more than once"));
        assert!(err("[header]\nstart = [\"bak\"]").contains("unknown variant"));
        assert!(err("[window]\ndecorations = \"fancy\"").contains("unknown variant"));
        assert!(err("[sidbar]\nvisible = false").contains("unknown field"));
    }

    #[test]
    fn auto_decorations_by_desktop() {
        assert_eq!(auto_decorations("GNOME"), Decorations::Full);
        assert_eq!(auto_decorations("ubuntu:GNOME"), Decorations::Full);
        assert_eq!(auto_decorations("KDE"), Decorations::Full);
        assert_eq!(auto_decorations("Hyprland"), Decorations::Minimal);
        assert_eq!(auto_decorations("sway"), Decorations::Minimal);
        assert_eq!(auto_decorations("niri"), Decorations::Minimal);
        assert_eq!(auto_decorations(""), Decorations::Full);
    }
}
