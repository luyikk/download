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
    // Must be first: catch silent panics and write a crash log + show dialog
    setup_panic_handler();

    // Load user config first to get log level
    let user_config = UserConfig::load();
    let log_buffer = gui_logger::init_gui_logger(user_config.log_level_filter());

    let icon = load_window_icon();

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 720.0])
        .with_min_inner_size([900.0, 500.0]);
    let viewport = if let Some(icon) = icon {
        viewport.with_icon(Arc::new(icon))
    } else {
        viewport
    };

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: build_wgpu_config(),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "DURL Download Manager",
        options,
        Box::new(move |cc| {
            Ok(Box::new(DurlApp::new(cc, log_buffer.clone(), user_config.clone())))
        }),
    ) {
        show_error(&format!("Failed to start: {e}"));
    }
}

/// Install a global panic hook that writes a crash log and shows an error dialog.
fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("{info}");

        // Write to crash.log next to user config
        let log_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("durl-gui")
            .join("crash.log");
        let _ = std::fs::create_dir_all(log_path.parent().unwrap());
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let content = format!("[{timestamp}]\n{msg}\n");
        let _ = std::fs::write(&log_path, &content);

        show_error(&format!(
            "Application crashed. Log written to:\n{}\n\n{msg}",
            log_path.display()
        ));
    }));
}

/// Show a modal error dialog (uses rfd which works without a running event loop).
fn show_error(msg: &str) {
    rfd::MessageDialog::new()
        .set_title("DURL - Error")
        .set_description(msg)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

/// Load the window icon from the embedded ICO file.
/// Returns None on any failure (non-fatal).
fn load_window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/letter-d.ico");
    let dir = ico::IconDir::read(std::io::Cursor::new(bytes as &[u8])).ok()?;
    let entry = dir
        .entries()
        .iter()
        .max_by_key(|e| e.width() * e.height())?;
    let image = entry.decode().ok()?;
    Some(egui::IconData {
        rgba: image.rgba_data().to_vec(),
        width: image.width(),
        height: image.height(),
    })
}

/// Build wgpu configuration with maximum compatibility.
///
/// Strategy:
///   1. Try any **hardware** adapter across all backends
///      (DX12, DX11, Vulkan, GL, Metal — whichever is available).
///   2. If nothing found, request a **software** fallback adapter
///      (WARP on Windows, llvmpipe on Linux).
///   3. If even that fails, return the default config and let eframe
///      handle it as usual.
fn build_wgpu_config() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu;
    use eframe::wgpu;

    // Instance with every backend enabled so we cover DX11 on Windows 10 VMs.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // 1. Hardware adapter (any backend, any power preference)
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    }));

    // 2. Software / WARP fallback
    let adapter = adapter.or_else(|| {
        log::info!("No hardware GPU found, trying software renderer (WARP / llvmpipe)");
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: true,
        }))
    });

    let Some(adapter) = adapter else {
        log::warn!("No wgpu adapter found at all, falling back to default config");
        return egui_wgpu::WgpuConfiguration::default();
    };

    let limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());

    let Ok((device, queue)) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            ..Default::default()
        },
        None,
    )) else {
        log::warn!("Failed to create wgpu device, falling back to default config");
        return egui_wgpu::WgpuConfiguration::default();
    };

    use std::sync::Arc;
    egui_wgpu::WgpuConfiguration {
        wgpu_setup: egui_wgpu::WgpuSetup::Existing {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
        },
        ..Default::default()
    }
}

