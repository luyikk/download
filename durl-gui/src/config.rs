use serde::{Deserialize, Serialize};

/// User configuration persisted in `user.toml`.
#[derive(Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_save_path")]
    pub default_save_path: String,
    #[serde(default = "default_task_count")]
    pub default_task_count: u64,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_language() -> String {
    "zh-CN".into()
}

fn default_save_path() -> String {
    // Try the system Downloads folder first (cross-platform via `dirs`)
    if let Some(p) = dirs::download_dir() {
        return p.display().to_string();
    }
    // Fall back to home directory
    if let Some(p) = dirs::home_dir() {
        return p.display().to_string();
    }
    // Last resort
    ".".into()
}

fn default_task_count() -> u64 {
    15
}

fn default_log_level() -> String {
    "Info".into()
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            default_save_path: default_save_path(),
            default_task_count: default_task_count(),
            log_level: default_log_level(),
        }
    }
}

impl UserConfig {
    /// Load user config from `user.toml` in the app config directory.
    pub fn load() -> Self {
        let path = crate::paths::user_config_path();
        match std::fs::read_to_string(&path) {
            Ok(data) => toml::from_str(&data).unwrap_or_default(),
            Err(_) => {
                let cfg = Self::default();
                cfg.save(); // Create default file on first run
                cfg
            }
        }
    }

    /// Save user config to `user.toml`.
    pub fn save(&self) {
        if let Ok(toml_str) = toml::to_string_pretty(self) {
            let _ = std::fs::write(crate::paths::user_config_path(), toml_str);
        }
    }

    /// Parse the log_level string into a `log::LevelFilter`.
    pub fn log_level_filter(&self) -> log::LevelFilter {
        match self.log_level.to_ascii_lowercase().as_str() {
            "off" => log::LevelFilter::Off,
            "error" => log::LevelFilter::Error,
            "warn" => log::LevelFilter::Warn,
            "info" => log::LevelFilter::Info,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            _ => log::LevelFilter::Info,
        }
    }
}

/// All valid log level names for the settings UI.
pub const LOG_LEVELS: &[&str] = &["Error", "Warn", "Info", "Debug", "Trace"];
