use std::path::PathBuf;

/// Returns the application config directory.
/// On Windows: `%APPDATA%/durl-gui-new/`
pub fn app_config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("durl-gui-new");
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

/// Directory where the browser extension files are extracted to.
pub fn extension_dir() -> PathBuf {
    let dir = app_config_dir().join("extension");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
