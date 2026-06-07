use dioxus::prelude::*;

use crate::components::log_panel::LogPanel;
use crate::components::sidebar::Sidebar;
use crate::components::theme_toggle::ThemeToggle;
use crate::components::toolbar::Toolbar;
use crate::state::app_state::AppState;
use crate::state::task::TaskStatus;
use crate::state::theme::ThemeClasses;
use crate::Route;

/// Shared layout wrapping all pages: Toolbar + Sidebar + Outlet + LogPanel.
#[component]
pub fn Shell() -> Element {
    let cls_ctx = use_context::<Signal<ThemeClasses>>();
    let cls = cls_ctx();

    let state = use_context::<AppState>();
    let log_collapsed = use_signal(|| false);

    // Extract signals before reading
    let mut tasks = state.tasks;
    let mut sel_id = state.selected_id;
    let mut logs = state.logs;

    // ── Derived counts ─────────────────────────────────────
    let all_count = tasks().len();
    let downloading_count = tasks()
        .iter()
        .filter(|t| {
            matches!(
                t.status,
                TaskStatus::Downloading | TaskStatus::Paused | TaskStatus::Starting
            )
        })
        .count();
    let completed_count = tasks()
        .iter()
        .filter(|t| matches!(t.status, TaskStatus::Completed | TaskStatus::Error))
        .count();
    let active_count = tasks()
        .iter()
        .filter(|t| t.status == TaskStatus::Downloading)
        .count();
    let total_speed: u64 = tasks()
        .iter()
        .filter(|t| t.status == TaskStatus::Downloading)
        .map(|t| t.speed)
        .sum();

    let sel_status = sel_id().and_then(|id| tasks().iter().find(|t| t.id == id).map(|t| t.status));
    let can_pause = sel_status == Some(TaskStatus::Downloading);
    let can_resume = sel_status == Some(TaskStatus::Paused);

    // ── Action handlers ────────────────────────────────────
    let handle_pause = move |_| {
        if let Some(id) = sel_id() {
            if let Some(idx) = tasks().iter().position(|t| t.id == id) {
                tasks.write()[idx].status = TaskStatus::Paused;
                logs.write().push(format!(
                    "[{}]  Paused: {}",
                    now_str(),
                    tasks()[idx].filename,
                ));
            }
        }
    };

    let handle_resume = move |_| {
        if let Some(id) = sel_id() {
            if let Some(idx) = tasks().iter().position(|t| t.id == id) {
                tasks.write()[idx].status = TaskStatus::Downloading;
                logs.write().push(format!(
                    "[{}]  Resumed: {}",
                    now_str(),
                    tasks()[idx].filename,
                ));
            }
        }
    };

    let handle_delete = move |_| {
        if let Some(id) = sel_id() {
            tasks.write().retain(|t| t.id != id);
            sel_id.set(None);
            logs.write()
                .push(format!("[{}]  Deleted task #{}", now_str(), id));
        }
    };

    rsx! {
        div { class: "flex flex-col h-screen {cls.page_bg} overflow-hidden",

            // ── Toolbar ───────────────────────────────────────
            Toolbar {
                active_count,
                total_speed,
                selected_id: sel_id(),
                can_pause,
                can_resume,
                on_pause: handle_pause,
                on_resume: handle_resume,
                on_delete: handle_delete,
                theme_toggle: rsx! { ThemeToggle {} },
            }

            // ── Body: Sidebar + Page content ──────────────────
            div { class: "flex flex-1 min-h-0",

                Sidebar {
                    filter: state.filter,
                    all_count,
                    downloading_count,
                    completed_count,
                }

                // Page content rendered by Router
                div { class: "flex-1 flex flex-col min-w-0 {cls.page_bg}",
                    Outlet::<Route> {}
                }
            }

            // ── Log Panel ─────────────────────────────────────
            LogPanel {
                logs,
                collapsed: log_collapsed,
            }
        }
    }
}

fn now_str() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
