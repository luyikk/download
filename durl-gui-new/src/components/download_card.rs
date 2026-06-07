use dioxus::prelude::*;

use crate::components::icons::file_type_for;
use crate::state::app_state::AppState;
use crate::state::download_task::{
    format_duration, format_size, format_speed, DownloadTask, TaskStatus,
};
use crate::state::i18n::LangStrings;
use crate::state::theme::ThemeClasses;

/// A single download task card.
#[component]
pub fn DownloadCard(
    task: DownloadTask,
    is_selected: bool,
    on_select: EventHandler<u64>,
) -> Element {
    let cls = use_context::<Signal<ThemeClasses>>();
    let cls = cls();
    let lang = use_context::<Signal<LangStrings>>();
    let lang = lang();
    let mut state = use_context::<AppState>();
    let file_type = file_type_for(&task.filename);

    let remaining = if task.speed > 0 && task.file_size > task.downloaded {
        let secs = (task.file_size - task.downloaded) / task.speed;
        format_duration(secs)
    } else {
        "—".into()
    };

    let border_class = if is_selected {
        "border-[#6C5CE7]/50 ring-0.5 ring-[#6C5CE7]/20"
    } else {
        "border-transparent"
    };

    let (status_icon, status_key, status_class) = match task.status {
        TaskStatus::Starting => ("⏳", "status.starting", "text-slate-400 bg-slate-500/10"),
        TaskStatus::Downloading => ("⬇", "status.downloading", "text-blue-400 bg-blue-500/10"),
        TaskStatus::Paused => ("⏸", "status.paused", "text-amber-400 bg-amber-500/10"),
        TaskStatus::Completed => (
            "✅",
            "status.completed",
            "text-emerald-400 bg-emerald-500/10",
        ),
        TaskStatus::Error => ("❌", "status.error", "text-red-400 bg-red-500/10"),
    };

    let status_text = lang.get(status_key);

    let progress_style = if task.status == TaskStatus::Completed {
        "background: #10b981"
    } else if task.status == TaskStatus::Error {
        "background: #ef4444"
    } else if task.status == TaskStatus::Paused {
        "background: #f59e0b"
    } else {
        "background: linear-gradient(to right, #3b82f6, #a855f7)"
    };
    let pulse_class = if task.status == TaskStatus::Downloading {
        "animate-shimmer"
    } else {
        ""
    };

    let left_label = lang.get("card.left");
    let completed_fmt = lang.get_fmt(
        "card.completed_in",
        &[("time", &format_duration(task.elapsed.as_secs()))],
    );
    let task_id = task.id;

    rsx! {
        div {
            class: "group flex items-stretch gap-0 mx-1 my-0.5 rounded-md \
                    {cls.card_bg} border {border_class} \
                    {cls.card_hover} hover:border-[#6C5CE7]/20 \
                    transition-all duration-200 cursor-pointer \
                    animate-slide-up",
            onclick: move |_| on_select(task_id),
            oncontextmenu: move |ev| {
                ev.prevent_default();
                let coords = ev.data().client_coordinates();
                (state.context_menu).set(Some((task_id, coords.x, coords.y)));
            },

            // Status indicator dot
            div { class: "flex items-center justify-center w-10 shrink-0",
                if task.status == TaskStatus::Downloading {
                    div { class: "w-2 h-2 rounded-full bg-blue-400 animate-pulse-glow" }
                } else if task.status == TaskStatus::Completed {
                    div { class: "w-2 h-2 rounded-full bg-emerald-400" }
                } else if task.status == TaskStatus::Error {
                    div { class: "w-2 h-2 rounded-full bg-red-400" }
                } else {
                    div { class: "w-2 h-2 rounded-full bg-amber-400" }
                }
            }

            // File type icon
            div { class: "flex items-center justify-center w-12 shrink-0",
                div { class: "w-9 h-9 rounded-xl {file_type.color_class} \
                              flex items-center justify-center text-lg",
                    "{file_type.emoji}"
                }
            }

            // Main content
            div { class: "flex-1 min-w-0 py-2.5 pr-4",
                div { class: "flex items-center justify-between gap-3 mb-1.5",
                    span { class: "{cls.text_primary} text-sm font-medium truncate",
                        "{task.filename}"
                    }
                    div { class: "flex items-center gap-1 px-2 py-0.5 rounded-lg {status_class} \
                                  text-xs font-medium shrink-0 translate-x-[-5px]",
                        span { "{status_icon}" }
                        span { "{status_text}" }
                    }
                }

                div { class: "mb-1.5",
                    div { class: "w-full h-1.5 bg-[#1e2430]/40 rounded-full overflow-hidden",
                        div {
                            class: "h-full rounded-full {pulse_class} transition-all duration-500 ease-out",
                            style: "{progress_style}; width: {task.progress}%",
                        }
                    }
                }

                div { class: "flex items-center gap-2 text-xs flex-wrap",
                    span { class: "{cls.text_secondary}",
                        "{format_size(task.downloaded)} / {format_size(task.file_size)}"
                    }
                    if task.status == TaskStatus::Downloading {
                        span { class: "{cls.text_muted}", "·" }
                        span { class: "text-blue-400 font-medium",
                            "{format_speed(task.speed)}"
                        }
                    }
                    if let Some(ref msg) = task.error_msg {
                        span { class: "{cls.text_muted}", "·" }
                        span { class: "text-red-400 truncate", "{msg}" }
                    }
                    if task.status == TaskStatus::Downloading {
                        span { class: "{cls.text_muted}", "·" }
                        span { class: "{cls.text_muted}",
                            "⏱ {remaining} {left_label}"
                        }
                    }
                    if task.status == TaskStatus::Completed {
                        span { class: "{cls.text_muted}", "·" }
                        span { class: "{cls.text_muted}", "{completed_fmt}" }
                    }

                    div { class: "flex-1" }
                    span { class: "{cls.text_muted} font-mono text-xs tabular-nums translate-x-[-5px]",
                        "{task.progress:.1}%"
                    }
                }
            }
        }
    }
}
