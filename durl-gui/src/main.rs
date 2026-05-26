#![windows_subsystem = "windows"]

mod app;
mod config;
mod gui_logger;
mod i18n;
pub mod paths;

use app::DurlApp;
use config::UserConfig;
use eframe::egui;
use std::sync::Arc;

fn main() {
    // Load user config first to get log level
    let user_config = UserConfig::load();
    let log_buffer = gui_logger::init_gui_logger(user_config.log_level_filter());

    let icon = load_window_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 720.0])
            .with_min_inner_size([900.0, 500.0])
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "DURL Download Manager",
        options,
        Box::new(move |cc| {
            Ok(Box::new(DurlApp::new(cc, log_buffer.clone(), user_config.clone())))
        }),
    ) {
        eprintln!("eframe error: {e}");
    }
}

/// Load the window icon from the embedded ICO file.
/// Picks the largest available image inside the ICO.
fn load_window_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/letter-d.ico");
    let dir = ico::IconDir::read(std::io::Cursor::new(bytes as &[u8]))
        .expect("failed to read letter-d.ico");

    // Pick the largest frame available
    let entry = dir.entries().iter()
        .max_by_key(|e| e.width() * e.height())
        .expect("ico has no entries");

    let image = entry.decode().expect("failed to decode ico entry");
    egui::IconData {
        rgba: image.rgba_data().to_vec(),
        width: image.width(),
        height: image.height(),
    }
}

