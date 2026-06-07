use dioxus::prelude::*;

use crate::state::download_task::{DownloadTask, Filter};

/// Shared application state, provided via context at the root.
#[derive(Clone, Copy)]
pub struct AppState {
    pub tasks: Signal<Vec<DownloadTask>>,
    pub filter: Signal<Filter>,
    pub selected_id: Signal<Option<u64>>,
    pub logs: Signal<Vec<String>>,
    pub show_new_dialog: Signal<bool>,
    /// Right-click context menu: (task_id, screen_x, screen_y)
    pub context_menu: Signal<Option<(u64, f64, f64)>>,
}
