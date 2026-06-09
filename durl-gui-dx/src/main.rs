#![windows_subsystem = "windows"]

use dashmap::DashMap;
use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

mod browser_server;
mod components;
mod ext_install;
mod gui_logger;
mod layout;
mod pages;
mod paths;
mod state;

use crate::state::app_state::{
    DeleteType, HandleDeleteType, HandlePauseType, HandleReDownloadType, HandleResumeType,
    NewDownLoadType, NewDownloadContext, PauseType, ReDownloadType, ResumeType,
};
use browser_server::{start_browser_server, BrowserDownloadReq};
use components::context_menu::ContextMenu;
use components::ext_install_dialog::ExtInstallDialog;
use gui_logger::LogBuffer;
use layout::shell::Shell;
use pages::downloads::Downloads;
use pages::new_download::NewDownload;
use pages::settings::Settings;
use state::app_state::AppState;
use state::config::UserConfig;
use state::download_task::{DownloadTask, Filter};
use state::i18n::LangStrings;
use state::theme::{DARK, LIGHT};

const MAIN_CSS: &str = include_str!("../assets/main.css");
const TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");

/// Application routes.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Shell)]
    #[route("/")]
    Downloads {},
    #[route("/settings")]
    Settings {},
}

/// Global tokio runtime — lazy-init, 2 workers.
pub fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}

/// Global browser download request receiver, polled by Shell on every tick.
static BROWSER_RX: OnceLock<Mutex<std::sync::mpsc::Receiver<BrowserDownloadReq>>> = OnceLock::new();

/// Retrieve the global browser receiver (if initialized).
pub fn try_recv_browser_req() -> Option<BrowserDownloadReq> {
    BROWSER_RX
        .get()
        .and_then(|mu| mu.lock().unwrap().try_recv().ok())
}

fn main() {
    // Disable always-on-top
    std::env::set_var("DIOXUS_ALWAYS_ON_TOP", "false");
    // Auto-detect when GPU compositing is unavailable on Linux.
    // WebKitGTK hardware rendering fails on software renderers, WSL,
    // and headless environments — falling back to software avoids a
    // blank white screen while keeping HW acceleration on real GPUs.
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    // Prime the runtime
    let _ = rt();

    dioxus::logger::init(Level::INFO).unwrap();
    #[cfg(feature = "desktop")]
    LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new().with_menu(None).with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("DUrl Download Manager")
                    .with_window_icon(Some(
                        dioxus::desktop::icon_from_memory(include_bytes!(
                            "../assets/favicon_16.png"
                        ))
                        .unwrap(),
                    ))
                    .with_inner_size(dioxus::desktop::LogicalSize::new(955.0, 630.0)),
            ),
        )
        .launch(App);
}

/// Root component.
#[component]
fn App() -> Element {
    let cfg = use_signal(UserConfig::load);

    // Initialize the GUI logger (channel-based, non-blocking)
    let log_buf: LogBuffer = use_signal(|| gui_logger::init_gui_logger(cfg().log_level_filter()))();
    use_context_provider(|| log_buf);

    let initial_theme = if cfg().theme == "light" { LIGHT } else { DARK };
    let theme = use_signal(|| initial_theme);
    use_context_provider(|| theme);

    let initial_lang = cfg().language.clone();
    let lang = use_signal(|| LangStrings::load(&initial_lang));
    use_context_provider(|| lang);

    // Start browser-extension HTTP server (once)
    BROWSER_RX.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<BrowserDownloadReq>();
        start_browser_server(tx);
        Mutex::new(rx)
    });

    let data = DownloadTask::load_all()?;

    // Load persisted tasks
    let tasks = use_signal(move || data);

    let filter = use_signal(|| Filter::All);
    let selected_id = use_signal(|| None::<u64>);
    let show_new_dialog = use_signal(|| false);
    let browser_req = use_signal(|| None::<BrowserDownloadReq>);
    let show_ext_install = use_signal(|| false);
    let ext_install_path = use_signal(String::new);
    let ext_browser_url = use_signal(String::new);
    let context_menu = use_signal(|| None::<(u64, f64, f64)>);
    let logs = use_signal(Vec::new);
    let dirty = use_signal(|| false);
    let sha256_queue = use_signal(DashMap::new);

    use_context_provider(|| AppState {
        tasks,
        filter,
        selected_id,
        logs,
        show_new_dialog,
        browser_req,
        show_ext_install,
        ext_install_path,
        ext_browser_url,
        context_menu,
        dirty,
        sha256_queue,
    });

    use_context_provider(|| cfg);

    use_future(|| async {
        loop {
            AppState::update().await;
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    });

    let new_down: NewDownLoadType = use_action(|data: NewDownloadContext| async move {
        AppState::new_download(data).await;
        dioxus::Ok(())
    });

    use_context_provider::<NewDownLoadType>(move || new_down);

    let pause: HandlePauseType = use_action(|id: PauseType| async move {
        AppState::handle_pause(id).await;
        dioxus::Ok(())
    });

    use_context_provider::<HandlePauseType>(move || pause);

    let resume = use_action(|id: ResumeType| async move {
        AppState::handle_resume(id).await;
        dioxus::Ok(())
    });

    use_context_provider::<HandleResumeType>(move || resume);

    let delete = use_action(|id: DeleteType| async move {
        AppState::handle_delete(id).await;
        dioxus::Ok(())
    });
    use_context_provider::<HandleDeleteType>(move || delete);

    let redownload = use_action(|id: ReDownloadType| async move {
        AppState::handle_redownload(id).await;
        dioxus::Ok(())
    });
    use_context_provider::<HandleReDownloadType>(move || redownload);

    // Log startup once — flows through GuiLogger → LogBuffer → LogPanel
    use_effect(|| {
        log::info!("DUrl v0.1.0 started");
    });

    rsx! {
        style { {MAIN_CSS} }
        style { {TAILWIND_CSS} }

        div {
            class: "h-screen w-screen overflow-hidden {theme().page_bg}",
            oncontextmenu: move |ev| ev.prevent_default(),

            Router::<Route> {}
            ContextMenu {}
            if show_new_dialog() {
                NewDownload {}
            }
            if show_ext_install() {
                ExtInstallDialog {}
            }
        }
    }
}
