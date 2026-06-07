use dioxus::prelude::*;

use crate::components::download_list::DownloadList;
use crate::state::app_state::AppState;
use crate::state::i18n::LangStrings;
use crate::state::task::{Filter, TaskStatus};
use crate::state::theme::ThemeClasses;

/// Main downloads page — the default route.
#[component]
pub fn Downloads() -> Element {
    let cls_ctx = use_context::<Signal<ThemeClasses>>();
    let cls = cls_ctx();
    let lang_ctx = use_context::<Signal<LangStrings>>();
    let lang = lang_ctx();

    let state = use_context::<AppState>();
    let tasks = state.tasks;
    let filter = state.filter;

    let all_count = tasks().len();
    let (downloading_count, completed_count) =
        tasks()
            .iter()
            .fold((0usize, 0usize), |(d, c), t| match t.status {
                TaskStatus::Downloading | TaskStatus::Paused | TaskStatus::Starting => (d + 1, c),
                TaskStatus::Completed | TaskStatus::Error => (d, c + 1),
            });

    let header_title = match filter() {
        Filter::All => lang.get("page_downloads.all"),
        Filter::Downloading => lang.get("page_downloads.downloading"),
        Filter::Completed => lang.get("page_downloads.completed"),
    };
    let header_sub = match filter() {
        Filter::All => lang.get_fmt(
            "page_downloads.all_sub",
            &[("count", &all_count.to_string())],
        ),
        Filter::Downloading => lang.get_fmt(
            "page_downloads.downloading_sub",
            &[("count", &downloading_count.to_string())],
        ),
        Filter::Completed => lang.get_fmt(
            "page_downloads.completed_sub",
            &[("count", &completed_count.to_string())],
        ),
    };

    rsx! {
        // Header
        div { class: "flex items-center justify-between px-4 py-3 \
                      {cls.border} border-b shrink-0",
            h1 { class: "flex items-center justify-start text-sm font-semibold {cls.text_primary} h-6 translate-x-2",
                "{header_title}"
            }
            span { class: "text-xs {cls.text_muted}",
                "{header_sub}"
            }
        }

        // Download cards
        DownloadList {
            tasks,
            filter,
            selected_id: state.selected_id,
        }
    }
}
