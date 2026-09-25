use std::fs;
use std::path::PathBuf;

use super::types::AppConfig;

// ─── Path Helper ───

/// Diptych's config directory: `~/.config/diptych`
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("diptych")
}

/// Returns the config file path: `~/.config/diptych/config.toml`
fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

// ─── Load ───

/// Reads config from disk. Returns `Default` if file is missing or invalid.
pub fn load_config() -> AppConfig {
    let path = config_path();

    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<AppConfig>(&content) {
                Ok(cfg) => {
                    println!("[config] Loaded from {:?}", path);
                    return cfg;
                }
                Err(e) => eprintln!("[config] Parse error, using defaults: {}", e),
            },
            Err(e) => eprintln!("[config] Read error, using defaults: {}", e),
        }
    } else {
        println!("[config] No config file found, using defaults.");
    }

    // First launch → write defaults
    let default = AppConfig::default();
    save_config(&default);
    default
}

// ─── Save ───

/// Persists the given config to disk as TOML.
pub fn save_config(config: &AppConfig) {
    let path = config_path();

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match toml::to_string_pretty(config) {
        Ok(content) => {
            if let Err(e) = fs::write(&path, &content) {
                eprintln!("[config] Failed to write: {}", e);
            } else {
                println!("[config] Saved to {:?}", path);
            }
        }
        Err(e) => eprintln!("[config] Serialization error: {}", e),
    }
}
