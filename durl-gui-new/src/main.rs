use dioxus::logger::tracing::Level;
use dioxus::prelude::*;

mod components;
mod layout;
mod pages;
mod paths;
mod state;

use components::context_menu::ContextMenu;
use layout::shell::Shell;
use pages::downloads::Downloads;
use pages::new_download::NewDownload;
use pages::settings::Settings;
use state::app_state::AppState;
use state::config::UserConfig;
use state::i18n::LangStrings;
use state::task::{mock_tasks, Filter};
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

fn main() {
    // Dioxus desktop defaults always_on_top=true, disable it
    std::env::set_var("DIOXUS_ALWAYS_ON_TOP", "false");

    dioxus::logger::init(Level::INFO).unwrap();
    LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new().with_menu(None))
        .launch(App);
}

/// Root component — loads config, provides global context, renders Router.
#[component]
fn App() -> Element {
    // ── Load persisted config ────────────────────────────────
    let cfg = use_signal(UserConfig::load);

    // ── Theme from config ────────────────────────────────────
    let initial_theme = if cfg().theme == "light" { LIGHT } else { DARK };
    let theme = use_signal(|| initial_theme);
    use_context_provider(|| theme);

    // ── Language from config ─────────────────────────────────
    let initial_lang = cfg().language.clone();
    let lang = use_signal(|| LangStrings::load(&initial_lang));
    use_context_provider(|| lang);

    // ── Application state ────────────────────────────────────
    let tasks = use_signal(mock_tasks);
    let filter = use_signal(|| Filter::All);
    let selected_id = use_signal(|| None::<u64>);
    let show_new_dialog = use_signal(|| false);
    let context_menu = use_signal(|| None::<(u64, f64, f64)>);
    let logs = use_signal(|| {
        vec![
            "[12:30:42]  DUrl v0.1.0 started".to_string(),
            "[12:30:42]  Listening for browser extension on 127.0.0.1:19283".to_string(),
            "[12:30:45]  Started downloading ubuntu-24.04-desktop-amd64.iso".to_string(),
            "[12:30:46]  Connected to releases.ubuntu.com".to_string(),
            "[12:31:10]  Started downloading music-collection.zip".to_string(),
            "[12:33:22]  vacation-photos-2024.zip: retry #2 successful".to_string(),
            "[12:35:01]  presentation.pptx: download completed in 12s".to_string(),
            "[12:36:18]  project-backup.tar.gz: ERROR — Connection timed out".to_string(),
        ]
    });

    use_context_provider(|| AppState {
        tasks,
        filter,
        selected_id,
        logs,
        show_new_dialog,
        context_menu,
    });

    // ── Config context (for settings page) ───────────────────
    use_context_provider(|| cfg);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}

        // ── Context menu (rendered above everything) ─────────
        ContextMenu {}

        // ── New Download dialog (modal overlay) ──────────────
        if show_new_dialog() {
            NewDownload {}
        }
    }
}
