use dioxus::prelude::*;

use crate::components::download_card::DownloadCard;
use crate::state::download_task::{DownloadTask, Filter, TaskStatus};
use crate::state::i18n::LangStrings;
use crate::state::theme::ThemeClasses;

/// List container for download cards with filtering.
#[component]
pub fn DownloadList(
    tasks: Signal<Vec<DownloadTask>>,
    filter: Signal<Filter>,
    selected_id: Signal<Option<u64>>,
) -> Element {
    let cls = use_context::<Signal<ThemeClasses>>()();
    let lang = use_context::<Signal<LangStrings>>();
    let lang = lang();

    let task_list = tasks.read();
    let filtered: Vec<&DownloadTask> = task_list
        .iter()
        .filter(|t| match filter() {
            Filter::All => true,
            Filter::Downloading => matches!(
                t.status,
                TaskStatus::Downloading | TaskStatus::Paused | TaskStatus::Starting
            ),
            Filter::Completed => matches!(t.status, TaskStatus::Completed | TaskStatus::Error),
        })
        .collect();

    if filtered.is_empty() {
        let (icon, empty_key) = match filter() {
            Filter::All => ("📭", "list.empty_all"),
            Filter::Downloading => ("⬇", "list.empty_downloading"),
            Filter::Completed => ("✅", "list.empty_completed"),
        };
        let empty_msg = lang.get(empty_key).to_string();
        let empty_hint = lang.get("list.empty_hint").to_string();

        return rsx! {
            div { class: "flex-1 flex flex-col items-center justify-center",
                span { class: "text-5xl mb-4 opacity-40", "{icon}" }
                p { class: "{cls.text_secondary} text-sm", "{empty_msg}" }
                p { class: "{cls.text_muted} text-xs mt-1", "{empty_hint}" }
            }
        };
    }

    rsx! {
        div { class: "flex-1 overflow-y-auto !py-1",
            for task in filtered {
                DownloadCard {
                    key: "{task.id}",
                    task: task.clone(),
                    is_selected: selected_id() == Some(task.id),
                    on_select: move |id: u64| selected_id.set(Some(id)),
                }
            }
        }
    }
}
