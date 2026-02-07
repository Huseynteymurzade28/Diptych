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

// ─── View Mode ───

/// Switches between grid (card), list (row), and graph (node) layouts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViewMode {
    Grid,
    List,
    Graph,
}

// ─── Application Config ───

/// All user-configurable settings, persisted to disk as TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // Appearance
    pub theme: String,
    pub icon_size: i32,
    pub view_mode: ViewMode,
    pub icon_theme: IconTheme,

    // Metadata display
    pub show_hidden: bool,
    pub show_file_size: bool,
    pub show_modified_date: bool,

    // Grouping
    pub grouping: GroupBy,

    // Window state
    pub window_width: i32,
    pub window_height: i32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "Catppuccin".to_string(),
            icon_size: 48,
            view_mode: ViewMode::Grid,
            icon_theme: IconTheme::Minimal,
            show_hidden: false,
            show_file_size: true,
            show_modified_date: true,
            grouping: GroupBy::None,
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
