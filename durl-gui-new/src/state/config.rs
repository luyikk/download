use serde::{Deserialize, Serialize};

/// User configuration persisted to `user.toml` on disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_save_path")]
    pub default_save_path: String,
    #[serde(default = "default_task_count")]
    pub default_task_count: u64,
}

fn default_language() -> String {
    "zh-CN".into()
}

fn default_theme() -> String {
    "dark".into()
}

fn default_save_path() -> String {
    if let Some(p) = dirs::download_dir() {
        return p.display().to_string();
    }
    if let Some(p) = dirs::home_dir() {
        return p.display().to_string();
    }
    ".".into()
}

fn default_task_count() -> u64 {
    8
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
            default_save_path: default_save_path(),
            default_task_count: default_task_count(),
        }
    }
}

impl UserConfig {
    /// Load from `user.toml`, falling back to defaults.
    pub fn load() -> Self {
        let path = crate::paths::user_config_path();
        match std::fs::read_to_string(&path) {
            Ok(data) => toml::from_str(&data).unwrap_or_default(),
            Err(_) => {
                let cfg = Self::default();
                cfg.save();
                cfg
            }
        }
    }

    /// Save to `user.toml`.
    pub fn save(&self) {
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = std::fs::write(crate::paths::user_config_path(), s);
        }
    }
}
