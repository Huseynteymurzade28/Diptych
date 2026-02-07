use serde::{Deserialize, Serialize};

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

/// Switches between grid (card) and list (row) layouts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViewMode {
    Grid,
    List,
}

// ─── Application Config ───

/// All user-configurable settings, persisted to disk as TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // Appearance
    pub theme: String,
    pub icon_size: i32,
    pub view_mode: ViewMode,

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
