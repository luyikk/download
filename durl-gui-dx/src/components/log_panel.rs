use crate::state::i18n::LangStrings;
use crate::state::log_entry::LogEntry;
use crate::state::theme::ThemeClasses;
use dioxus::prelude::*;
use log::Level;

/// Tailwind CSS color class for a log level tag.
fn level_color(level: Level) -> &'static str {
    match level {
        Level::Trace => "text-slate-500",
        Level::Debug => "text-slate-400",
        Level::Info => "text-sky-400",
        Level::Warn => "text-amber-400",
        Level::Error => "text-red-400",
    }
}

/// Bottom log panel, collapsible. Shows structured `LogEntry` items with
/// color-coded level tags.
#[component]
pub fn LogPanel(logs: Signal<Vec<LogEntry>>, collapsed: Signal<bool>) -> Element {
    let cls = use_context::<Signal<ThemeClasses>>()();
    let lang = use_context::<Signal<LangStrings>>()();

    let title = lang.get_fmt("log.title", &[("count", &logs().len().to_string())]);
    let clear_label = lang.get("log.clear").to_string();
    let empty_label = lang.get("log.empty").to_string();

    if collapsed() {
        return rsx! {
            div {
                class: "shrink-0 border-t {cls.border} {cls.sidebar_bg} \
                        cursor-pointer {cls.hover_bg} transition-colors duration-150",
                onclick: move |_| collapsed.set(false),
                div { class: "flex items-center gap-2 px-4 py-1.5",
                    span { class: "text-xs {cls.text_muted}", "▶" }
                    span { class: "text-xs {cls.text_muted} font-medium uppercase tracking-wider",
                        "{title}"
                    }
                }
            }
        };
    }

    // Pre-compute display strings so rsx! stays simple
    let entries: Vec<_> = logs()
        .iter()
        .map(|e| {
            let show_level = e.level != Level::Info;
            let level_tag = if show_level {
                format!("[{:<5}]", e.level.to_string())
            } else {
                String::new()
            };
            let color = level_color(e.level);
            (
                e.time.clone(),
                show_level,
                level_tag,
                color,
                e.message.clone(),
            )
        })
        .collect();

    rsx! {
        div {
            class: "shrink-0 border-t {cls.border} {cls.panel_bg} flex flex-col",
            style: "height: 150px",

            div {
                class: "flex items-center justify-between px-4 py-1.5 \
                        cursor-pointer {cls.hover_bg} transition-colors duration-150 \
                        border-b {cls.border} h-5",
                onclick: move |_| collapsed.set(true),
                div { class: "flex items-center gap-2",
                    span { class: "text-xs {cls.text_muted}", "▼" }
                    span { class: "text-xs {cls.text_muted} font-medium uppercase tracking-wider",
                        "{title}"
                    }
                }
                button {
                    class: "text-xs {cls.text_muted} hover:{cls.text_secondary} transition-colors relative right-2",
                    onclick: move |e| {
                        e.stop_propagation();
                        logs.write().clear();
                    },
                    "{clear_label}"
                }
            }

            div {
                class: "flex-1 overflow-y-auto !px-2 !py-1 font-mono text-xs leading-relaxed",
                if entries.is_empty() {
                    div { class: "{cls.text_muted} italic",
                        "{empty_label}"
                    }
                } else {
                    for (time, show_level, level_tag, color, msg) in entries {
                        div { class: "{cls.text_muted} py-px whitespace-nowrap",
                            span { class: "text-slate-600", "[{time}]" }
                            if show_level {
                                span { class: "text-slate-600", " " }
                                span { class: "font-medium {color}", "{level_tag}" }
                            }
                            span { " {msg}" }
                        }
                    }
                }
            }
        }
    }
}
