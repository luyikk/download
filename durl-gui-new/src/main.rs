use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
use std::sync::OnceLock;

mod components;
mod gui_logger;
mod layout;
mod pages;
mod paths;
mod state;

use components::context_menu::ContextMenu;
use gui_logger::LogBuffer;
use layout::shell::Shell;
use pages::downloads::Downloads;
use pages::new_download::NewDownload;
use pages::settings::Settings;
use state::app_state::AppState;
use state::config::UserConfig;
use state::download_task::{DownloadTask, Filter};
use state::i18n::LangStrings;
use state::log_entry::LogEntry;
use state::theme::{DARK, LIGHT};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

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
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}

fn main() {
    // Disable always-on-top
    std::env::set_var("DIOXUS_ALWAYS_ON_TOP", "false");
    // Prime the runtime
    let _ = rt();

    dioxus::logger::init(Level::INFO).unwrap();
    #[cfg(feature = "desktop")]
    LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new().with_menu(None))
        .launch(App);
}

/// Root component.
#[component]
fn App() -> Element {
    info!("start");

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

    // Load persisted tasks
    let tasks = use_loader(|| async move { dioxus::Ok(DownloadTask::load_all()?) })?;

    let filter = use_signal(|| Filter::All);
    let selected_id = use_signal(|| None::<u64>);
    let show_new_dialog = use_signal(|| false);
    let context_menu = use_signal(|| None::<(u64, f64, f64)>);
    // Initial log entry (app-level, no level tag)
    let logs = use_signal(|| vec![LogEntry::app("DUrl v0.1.0 started")]);
    let dirty = use_signal(|| false);

    use_context_provider(|| AppState {
        tasks,
        filter,
        selected_id,
        logs,
        show_new_dialog,
        context_menu,
        dirty,
    });

    use_context_provider(|| cfg);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div {
            class: "h-screen w-screen overflow-hidden {theme().page_bg}",
            oncontextmenu: move |ev| ev.prevent_default(),

            Router::<Route> {}
            ContextMenu {}
            if show_new_dialog() {
                NewDownload {}
            }
        }
    }
}
