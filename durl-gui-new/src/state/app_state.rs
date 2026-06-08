use dioxus::fullstack::Loader;
use dioxus::prelude::*;

use crate::browser_server::BrowserDownloadReq;
use crate::state::download_task::{DownloadTask, Filter};
use crate::state::log_entry::LogEntry;

/// Shared application state, provided via context at the root.
#[derive(Clone, Copy)]
pub struct AppState {
    pub tasks: Loader<Vec<DownloadTask>>,
    pub filter: Signal<Filter>,
    pub selected_id: Signal<Option<u64>>,
    pub logs: Signal<Vec<LogEntry>>,
    pub show_new_dialog: Signal<bool>,
    /// Pre-fill data for the NewDownload dialog (from browser extension).
    pub browser_req: Signal<Option<BrowserDownloadReq>>,
    /// Right-click context menu: (task_id, screen_x, screen_y)
    pub context_menu: Signal<Option<(u64, f64, f64)>>,
    /// Dirty flag — set true when tasks change, triggers auto-save.
    pub dirty: Signal<bool>,
}
