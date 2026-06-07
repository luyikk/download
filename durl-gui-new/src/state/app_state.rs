use dioxus::prelude::*;

use crate::state::task::{Filter, MockTask};

/// Shared application state, provided via context at the root.
#[derive(Clone, Copy)]
pub struct AppState {
    pub tasks: Signal<Vec<MockTask>>,
    pub filter: Signal<Filter>,
    pub selected_id: Signal<Option<u64>>,
    pub logs: Signal<Vec<String>>,
    pub show_new_dialog: Signal<bool>,
}
