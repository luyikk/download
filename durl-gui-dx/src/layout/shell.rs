use crate::components::log_panel::LogPanel;
use crate::components::sidebar::Sidebar;
use crate::components::theme_toggle::ThemeToggle;
use crate::components::toolbar::Toolbar;
use crate::state::app_state::{AppState, HandleDeleteType, HandlePauseType, HandleResumeType};
use crate::state::download_task::{DownloadTask, TaskStatus};
use crate::state::theme::ThemeClasses;
use crate::Route;
use dioxus::prelude::*;

/// Shared layout wrapping all pages: Toolbar + Sidebar + Outlet + LogPanel.
#[component]
pub fn Shell() -> Element {
    let cls = use_context::<Signal<ThemeClasses>>()();
    let state = use_context::<AppState>();
    let log_collapsed = use_signal(|| false);

    // Extract signals
    let tasks_sig = state.tasks;
    let mut select_id_signal = state.selected_id;
    let logs = state.logs;
    let filter = state.filter;
    let mut dirty = state.dirty;

    // ── Update tasks ───────────────────────────────────────
    let tasks = tasks_sig.read();

    // ── Derived counts ─────────────────────────────────────
    let all_count = tasks.len();
    let downloading_count = tasks
        .iter()
        .filter(|t| {
            matches!(
                t.status,
                TaskStatus::Downloading | TaskStatus::Paused | TaskStatus::Starting
            )
        })
        .count();
    let completed_count = tasks
        .iter()
        .filter(|t| matches!(t.status, TaskStatus::Completed | TaskStatus::Error))
        .count();
    let active_count = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Downloading)
        .count();
    let total_speed: u64 = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Downloading)
        .map(|t| t.speed)
        .sum();

    let select_id = select_id_signal();
    let sel_status = select_id.and_then(|id| tasks.iter().find(|t| t.id == id).map(|t| t.status));

    let can_pause = sel_status == Some(TaskStatus::Downloading);
    let can_resume = sel_status == Some(TaskStatus::Paused);

    // ── Auto-save dirty tasks ───────────────────────────────
    if dirty() {
        let task_list = tasks_sig.read();
        DownloadTask::save_all(&task_list)?;
        dirty.set(false);
    }

    rsx! {
        div { class: "flex flex-col h-screen {cls.page_bg} overflow-hidden",

            Toolbar {
                active_count,
                total_speed,
                selected_id: select_id,
                can_pause,
                can_resume,
                on_pause: move|_|{
                    if let Some(selected_id) = select_id_signal() {
                        consume_context::<HandlePauseType>().call(selected_id.into());
                    }
                },
                on_resume: move|_|{
                    if let Some(selected_id) = select_id_signal() {
                        consume_context::<HandleResumeType>().call(selected_id.into());
                    }
                },
                on_delete: move|_|{
                    if let Some(selected_id) = select_id_signal() {
                        consume_context::<HandleDeleteType>().call(selected_id.into());
                        select_id_signal.set(None);
                    }
                },
                theme_toggle: rsx! { ThemeToggle {} },
            }

            div { class: "flex flex-1 min-h-0",
                Sidebar { filter, all_count, downloading_count, completed_count }
                div { class: "flex-1 flex flex-col min-w-0 {cls.page_bg}",
                    Outlet::<Route> {}
                }
            }

            LogPanel { logs, collapsed: log_collapsed }
        }
    }
}
