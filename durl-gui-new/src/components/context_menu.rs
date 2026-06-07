use dioxus::prelude::*;

use crate::state::app_state::AppState;
use crate::state::i18n::LangStrings;
use crate::state::task::TaskStatus;
use crate::state::theme::ThemeClasses;

/// Right-click context menu for download cards.
#[component]
pub fn ContextMenu() -> Element {
    let cls = use_context::<Signal<ThemeClasses>>();
    let cls = cls();
    let lang = use_context::<Signal<LangStrings>>();
    let lang = lang();
    let mut state = use_context::<AppState>();

    let menu = (state.context_menu)();
    if menu.is_none() {
        return rsx! {};
    }
    let (task_id, x, y) = menu.unwrap();
    let task = (state.tasks)().iter().find(|t| t.id == task_id).cloned();
    if task.is_none() {
        (state.context_menu).set(None);
        return rsx! {};
    }
    let task = task.unwrap();

    let close = move |_| {
        (state.context_menu).set(None);
    };

    rsx! {
        // Invisible backdrop to close on click outside
        div {
            class: "fixed inset-0 z-[60]",
            onclick: close,
            oncontextmenu: move |e| {
                e.stop_propagation();
                (state.context_menu).set(None);
            },

            // The menu
            div {
                class: "fixed z-[61] min-w-[200px] py-1.5 rounded-lg {cls.card_bg} \
                        border {cls.border} shadow-xl shadow-black/30",
                style: "left: {x}px; top: {y}px;",
                onclick: |e| e.stop_propagation(),
                oncontextmenu: |e| e.stop_propagation(),

                // ── Copy URL ─────────────────────────────────
                {
                    rsx! {
                        ContextMenuItem {
                            label: lang.get("context_menu.copy_url").to_string(),
                            enabled: true,
                            onaction: move |_| {
                                (state.context_menu).set(None);
                                // TODO: clipboard copy
                            },
                        }
                    }
                }

                // Separator
                div { class: "my-1 border-t {cls.border}" }

                // ── Pause ────────────────────────────────────
                if task.status == TaskStatus::Downloading {
                    {
                        let id = task.id;
                        rsx! {
                            ContextMenuItem {
                                label: lang.get("context_menu.pause").to_string(),
                                enabled: true,
                                onaction: move |_| {
                                    if let Some(idx) = (state.tasks)().iter().position(|t| t.id == id) {
                                        (state.tasks).write()[idx].status = TaskStatus::Paused;
                                        (state.logs).write().push(format!(
                                            "[{}]  Paused: {}", now_str(), (state.tasks)()[idx].filename,
                                        ));
                                    }
                                    (state.context_menu).set(None);
                                },
                            }
                        }
                    }
                }

                // ── Resume ───────────────────────────────────
                if task.status == TaskStatus::Paused {
                    {
                        let id = task.id;
                        rsx! {
                            ContextMenuItem {
                                label: lang.get("context_menu.resume").to_string(),
                                enabled: true,
                                onaction: move |_| {
                                    if let Some(idx) = (state.tasks)().iter().position(|t| t.id == id) {
                                        (state.tasks).write()[idx].status = TaskStatus::Downloading;
                                        (state.logs).write().push(format!(
                                            "[{}]  Resumed: {}", now_str(), (state.tasks)()[idx].filename,
                                        ));
                                    }
                                    (state.context_menu).set(None);
                                },
                            }
                        }
                    }
                }

                // ── Completed actions ────────────────────────
                if task.status == TaskStatus::Completed {
                    ContextMenuItem {
                        label: lang.get("context_menu.open_file").to_string(),
                        enabled: true,
                        onaction: move |_| { (state.context_menu).set(None); },
                    }
                    ContextMenuItem {
                        label: lang.get("context_menu.open_dir").to_string(),
                        enabled: true,
                        onaction: move |_| { (state.context_menu).set(None); },
                    }
                    ContextMenuItem {
                        label: lang.get("context_menu.redownload").to_string(),
                        enabled: true,
                        onaction: move |_| { (state.context_menu).set(None); },
                    }
                }

                // Separator
                div { class: "my-1 border-t {cls.border}" }

                // ── Delete ───────────────────────────────────
                {
                    let id = task.id;
                    rsx! {
                        ContextMenuItem {
                            label: lang.get("context_menu.delete").to_string(),
                            enabled: true,
                            onaction: move |_| {
                                (state.tasks).write().retain(|t| t.id != id);
                                (state.selected_id).set(None);
                                (state.logs).write().push(
                                    format!("[{}]  Deleted task #{}", now_str(), id),
                                );
                                (state.context_menu).set(None);
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Single context menu item button.
#[component]
fn ContextMenuItem(label: String, enabled: bool, onaction: EventHandler<()>) -> Element {
    let cls = use_context::<Signal<ThemeClasses>>();
    let cls = cls();

    rsx! {
        button {
            class: "w-full text-left px-3 py-1.5 text-sm \
                    {cls.text_secondary} hover:{cls.text_primary} \
                    hover:bg-[#6C5CE7]/10 transition-colors \
                    disabled:opacity-30 disabled:pointer-events-none",
            disabled: !enabled,
            onclick: move |_| onaction(()),
            "{label}"
        }
    }
}

fn now_str() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60,
    )
}
