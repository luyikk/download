use dioxus::prelude::*;

use crate::state::i18n::LangStrings;
use crate::state::theme::ThemeClasses;

/// Bottom log panel, collapsible.
#[component]
pub fn LogPanel(logs: Signal<Vec<String>>, collapsed: Signal<bool>) -> Element {
    let cls_ctx = use_context::<Signal<ThemeClasses>>();
    let cls = cls_ctx();
    let lang_ctx = use_context::<Signal<LangStrings>>();
    let lang = lang_ctx();

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

    rsx! {
        div {
            class: "shrink-0 border-t {cls.border} {cls.panel_bg} flex flex-col",
            style: "height: 120px",

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

            div { class: "flex-1 overflow-y-auto px-4 py-2 font-mono text-xs leading-relaxed",
                if logs().is_empty() {
                    div { class: "{cls.text_muted} italic",
                        "{empty_label}"
                    }
                } else {
                    for entry in logs() {
                        div { class: "{cls.text_muted} py-px", "{entry}" }
                    }
                }
            }
        }
    }
}
