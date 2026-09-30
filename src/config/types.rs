use serde::{Deserialize, Serialize};

// ─── Icon Theme ───

/// Determines which icon set to use for file/folder display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IconTheme {
    Minimal,
    Colorful,
    Outline,
}

impl IconTheme {
    pub fn all_names() -> Vec<&'static str> {
        vec!["Minimal", "Colorful", "Outline"]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            IconTheme::Minimal => "Minimal",
            IconTheme::Colorful => "Colorful",
            IconTheme::Outline => "Outline",
        }
    }

    pub fn from_name(name: &str) -> IconTheme {
        match name {
            "Colorful" => IconTheme::Colorful,
            "Outline" => IconTheme::Outline,
            _ => IconTheme::Minimal,
        }
    }
}

// ─── Grouping Strategy ───

/// Determines how files are grouped in the content view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GroupBy {
    None,
    Type,
    Date,
    Name,
}

// ─── Sorting ───

/// Sort key for the grid and list views. Folders always come first.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum SortBy {
    #[default]
    Name,
    Size,
    Modified,
    Type,
}

impl SortBy {
    pub const ALL: [(SortBy, &'static str, &'static str); 4] = [
        (SortBy::Name, "name", "Name"),
        (SortBy::Size, "size", "Size"),
        (SortBy::Modified, "modified", "Modified"),
        (SortBy::Type, "type", "Type"),
    ];

    pub fn id(self) -> &'static str {
        Self::ALL.iter().find(|(s, _, _)| *s == self).unwrap().1
    }

    pub fn from_id(id: &str) -> Option<SortBy> {
        Self::ALL
            .iter()
            .find(|(_, i, _)| *i == id)
            .map(|(s, _, _)| *s)
    }
}

// ─── View Mode ───

/// Switches between grid (card), list (row), graph (node), and tree (hierarchy) layouts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViewMode {
    Grid,
    List,
    Graph,
    Tree,
}

// ─── Click Behavior ───

/// Whether a single click opens items, or selects them (double click opens).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OpenWith {
    SingleClick,
    DoubleClick,
}

// ─── Application Config ───

/// All user-configurable settings, persisted to disk as TOML.
/// Missing keys fall back to `Default`, so older/partial files still load.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    // Appearance
    /// Legacy theme name (pre-`theme.toml`); only used once to seed `theme.toml`.
    pub theme: String,
    pub icon_size: i32,
    pub view_mode: ViewMode,
    pub icon_theme: IconTheme,

    // Metadata display
    pub show_hidden: bool,
    pub show_file_size: bool,
    pub show_modified_date: bool,

    // Grouping & sorting
    pub grouping: GroupBy,
    pub sort_by: SortBy,
    pub sort_descending: bool,

    // Behavior
    pub open_with: OpenWith,

    // Window state
    pub window_width: i32,
    pub window_height: i32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "Hearth".to_string(),
            icon_size: 48,
            view_mode: ViewMode::Grid,
            icon_theme: IconTheme::Minimal,
            show_hidden: false,
            show_file_size: true,
            show_modified_date: true,
            grouping: GroupBy::None,
            sort_by: SortBy::Name,
            sort_descending: false,
            open_with: OpenWith::DoubleClick,
            window_width: 1100,
            window_height: 700,
        }
    }
}

impl AppConfig {
    /// Load config from disk (convenience wrapper).
    pub fn load() -> Self {
        super::persistence::load_config()
    }

    /// Persist this config to disk (convenience wrapper).
    pub fn save(&self) {
        super::persistence::save_config(self);
    }
}
