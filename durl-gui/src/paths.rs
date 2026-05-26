use std::path::PathBuf;

/// Returns the application config directory.
/// On Windows: `%APPDATA%/durl-gui/`
/// On Linux:   `~/.config/durl-gui/`
/// On macOS:   `~/Library/Application Support/durl-gui/`
pub fn app_config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("durl-gui");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Path to `user.toml` in the app config directory.
pub fn user_config_path() -> PathBuf {
    app_config_dir().join("user.toml")
}

/// Path to `durl-gui-tasks.json` in the app config directory.
pub fn tasks_config_path() -> PathBuf {
    app_config_dir().join("durl-gui-tasks.json")
}

/// Directory containing language TOML files.
pub fn lang_dir() -> PathBuf {
    let dir = app_config_dir().join("lang");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
