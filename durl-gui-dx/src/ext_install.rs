//! Browser extension file extraction and browser launcher.

use std::path::PathBuf;

/// Which browser to launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserKind {
    Chrome,
    Edge,
}

/// Extract the bundled browser extension files to the app config directory.
/// Returns the directory path on success.
pub fn extract_extension_files() -> Result<PathBuf, std::io::Error> {
    let dir = crate::paths::extension_dir();
    let icons_dir = dir.join("icons");
    std::fs::create_dir_all(&icons_dir)?;

    std::fs::write(
        dir.join("manifest.json"),
        include_bytes!("../../extension/manifest.json"),
    )?;
    std::fs::write(
        dir.join("background.js"),
        include_bytes!("../../extension/background.js"),
    )?;
    std::fs::write(
        icons_dir.join("icon16.png"),
        include_bytes!("../../extension/icons/icon16.png"),
    )?;
    std::fs::write(
        icons_dir.join("icon48.png"),
        include_bytes!("../../extension/icons/icon48.png"),
    )?;
    std::fs::write(
        icons_dir.join("icon128.png"),
        include_bytes!("../../extension/icons/icon128.png"),
    )?;

    log::info!("[ext] Extension extracted to: {}", dir.display());
    Ok(dir)
}

/// Launch the browser without navigating to any URL.
/// The user will copy the extensions URL from the dialog and paste it manually.
pub fn launch_browser(kind: BrowserKind) {
    #[cfg(windows)]
    {
        let browser = match kind {
            BrowserKind::Chrome => "chrome",
            BrowserKind::Edge => "msedge",
        };
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", browser])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let app = match kind {
            BrowserKind::Chrome => "Google Chrome",
            BrowserKind::Edge => "Microsoft Edge",
        };
        let _ = std::process::Command::new("open").args(["-a", app]).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let browser = match kind {
            BrowserKind::Chrome => "google-chrome",
            BrowserKind::Edge => "microsoft-edge",
        };
        let _ = std::process::Command::new(browser).spawn();
    }
}
