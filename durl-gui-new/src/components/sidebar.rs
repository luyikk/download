use dioxus::prelude::*;

use crate::state::download_task::Filter;
use crate::state::i18n::LangStrings;
use crate::state::theme::ThemeClasses;
use crate::Route;

/// Sidebar navigation with filter tabs and a Settings link.
#[component]
pub fn Sidebar(
    filter: Signal<Filter>,
    all_count: usize,
    downloading_count: usize,
    completed_count: usize,
) -> Element {
    let cls_ctx = use_context::<Signal<ThemeClasses>>();
    let cls = cls_ctx();
    let lang_ctx = use_context::<Signal<LangStrings>>();
    let lang = lang_ctx();

    let items = [
        (Filter::All, "📋", lang.get("sidebar.all"), all_count),
        (
            Filter::Downloading,
            "⬇",
            lang.get("sidebar.downloading"),
            downloading_count,
        ),
        (
            Filter::Completed,
            "✅",
            lang.get("sidebar.completed"),
            completed_count,
        ),
    ];

    let current_route = use_route::<Route>();
    let on_settings_page = matches!(current_route, Route::Settings {});

    let sidebar_title = lang.get("sidebar.downloads").to_string();
    let settings_label = lang.get("sidebar.settings").to_string();
    let downloads_label = lang.get("sidebar.downloads").to_string();

    rsx! {
        div {
            class: "w-56 shrink-0 {cls.sidebar_bg} border-r {cls.border} \
                    flex flex-col select-none h-full",

            div { class: "px-4 py-4",
                h2 { class: "flex items-center text-xs font-semibold {cls.text_muted} uppercase tracking-widest h-6 translate-x-1",
                    "{sidebar_title}"
                }
            }

            div { class: "flex flex-col gap-0.5 px-3",
                for (f, icon, label, count) in items.iter() {
                    {
                        let f = *f;
                        let is_active = filter() == f && !on_settings_page;
                        let item_class: String = if is_active {
                            format!(
                                "{} {} {} border-l-2 {}",
                                cls.active_item_bg, cls.active_item_text,
                                cls.active_item_accent, cls.border,
                            )
                        } else {
                            format!(
                                "{} {} hover:{} border-l-2 border-l-transparent",
                                cls.text_secondary, cls.hover_bg, cls.text_primary,
                            )
                        };

                        rsx! {
                            button {
                                class: "flex items-center gap-3 px-3 py-2 rounded-r-lg \
                                        text-sm transition-all duration-150 {item_class}",
                                onclick: move |_| filter.set(f),

                                span { class: "text-base leading-none w-5 text-center", "{icon}" }
                                span { class: "flex-1 text-left font-medium",
                                    "{label}"
                                }
                                if *count > 0 {
                                    span { class: "text-xs px-1.5 py-0.5 rounded-md \
                                                  {cls.badge_bg} {cls.badge_text} \
                                                  font-mono min-w-[1.5rem] text-center",
                                        "{count}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Spacer
            div { class: "flex-1" }

            // Settings link at bottom
            div { class: "border-t {cls.border}" }
            Link {
                class: "flex items-center gap-3 px-3 py-3 mx-3 rounded-lg \
                        text-sm transition-all duration-150 \
                        {cls.text_secondary} {cls.hover_bg} hover:{cls.text_primary}",
                to: if on_settings_page { Route::Downloads {} } else { Route::Settings {} },
                span { class: "text-base leading-none w-5 text-center",
                    if on_settings_page { "📋" } else { "⚙" }
                }
                span { class: "flex-1 text-left font-medium",
                    if on_settings_page { "{downloads_label}" } else { "{settings_label}" }
                }
            }

        }
    }
}
