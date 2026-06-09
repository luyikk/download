use dioxus::prelude::*;

use crate::state::app_state::AppState;
use crate::state::download_task::format_speed;
use crate::state::i18n::LangStrings;
use crate::state::theme::ThemeClasses;
use crate::Route;

/// Top toolbar: action buttons + active stats.
#[component]
pub fn Toolbar(
    active_count: usize,
    total_speed: u64,
    selected_id: Option<u64>,
    can_pause: bool,
    can_resume: bool,
    on_pause: EventHandler<()>,
    on_resume: EventHandler<()>,
    on_delete: EventHandler<()>,
    theme_toggle: Element,
) -> Element {
    let cls = use_context::<Signal<ThemeClasses>>()();
    let lang = use_context::<Signal<LangStrings>>()();
    let mut state = use_context::<AppState>();

    let speed_str = format_speed(total_speed);
    let nav = navigator();

    let lbl_new = lang.get("toolbar.new").to_string();
    let lbl_pause = lang.get("toolbar.pause").to_string();
    let lbl_resume = lang.get("toolbar.resume").to_string();
    let lbl_delete = lang.get("toolbar.delete").to_string();
    let active_text = lang.get_fmt("toolbar.active", &[("count", &active_count.to_string())]);

    rsx! {
        div {
            class: "flex items-center justify-between h-14 px-4 \
                    {cls.sidebar_bg} border-b {cls.border} shrink-0 select-none \
                    backdrop-blur-xl ",

            // Left: Logo + action buttons
            div { class: "flex items-center gap-2",

                // Logo — click to go home
                div {
                    class: "flex items-center gap-2 mr-3 cursor-pointer",
                    onclick: move |_| { let _ = nav.push(Route::Downloads {}); },
                    div { class: "translate-x-[4px] w-7 h-7 rounded-lg bg-gradient-to-br {cls.logo_gradient} \
                                  flex items-center justify-center {cls.logo_text} \
                                  font-bold text-xs shadow-sm",
                        "D"
                    }
                    span { class: "{cls.text_primary} font-bold font-semibold text-sm tracking-wide !px-1",
                        "DUrl"
                    }
                }

                // Separator
                div { class: "w-px h-6 {cls.divider}" }

                // New Download
                button {
                    class: "flex items-center !gap-1 min-w-[50px] !px-3 !py-1.5 rounded-lg text-sm font-medium \
                            border {cls.border} {cls.text_secondary} \
                            {cls.hover_bg} hover:text-green-500 \
                            transition-all duration-150 active:scale-95",
                    onclick: move |_| state.show_new_dialog.set(true),
                    span { "+" }
                    span { "{lbl_new}" }
                }

                // Separator
                div { class: "w-px h-6 {cls.divider}" }

                // Pause
                button {
                    class: "flex items-center !gap-1.5 !px-3 !py-1.5 rounded-lg \
                            {cls.text_secondary} text-sm font-medium \
                            {cls.hover_bg} hover:text-amber-500 \
                            transition-all duration-150 active:scale-95 \
                            disabled:opacity-30 disabled:pointer-events-none",
                    disabled: !can_pause,
                    onclick: move |_| on_pause(()),
                    span { "⏸" }
                    span { "{lbl_pause}" }
                }

                // Resume
                button {
                    class: "flex items-center !gap-1.5 !px-3 !py-1.5 rounded-lg \
                            {cls.text_secondary} text-sm font-medium \
                            {cls.hover_bg} hover:text-emerald-500 \
                            transition-all duration-150 active:scale-95 \
                            disabled:opacity-30 disabled:pointer-events-none",
                    disabled: !can_resume,
                    onclick: move |_| on_resume(()),
                    span { "▶" }
                    span { "{lbl_resume}" }
                }

                // Delete
                button {
                    class: "flex items-center !gap-1.5 !px-3 !py-1.5 rounded-lg \
                            {cls.text_secondary} text-sm font-medium \
                            {cls.hover_bg} hover:text-red-400 \
                            transition-all duration-150 active:scale-95 \
                            disabled:opacity-30 disabled:pointer-events-none",
                    disabled: selected_id.is_none(),
                    onclick: move |_| on_delete(()),
                    span { "🗑" }
                    span { "{lbl_delete}" }
                }
            }

            // Right: Stats + theme toggle + settings
            div { class: "flex items-center gap-5 relative right-[10px]",
                if active_count > 0 {
                    div { class: "flex items-center gap-6 px-3 py-1 rounded-lg \
                                  {cls.card_bg} border {cls.border}",
                        span { class: "w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse-glow self-center relative left-[10px]" }
                        span { class: "{cls.text_muted} text-xs",
                            "{active_text}"
                        }
                        span { class: "{cls.text_muted}", "·" }
                        span { class: "{cls.text_secondary} text-xs font-medium relative right-[10px]",
                            "{speed_str}"
                        }
                    }
                }

                // Theme toggle
                {theme_toggle}

                // Settings — navigates to /settings page
                button {
                    class: "p-3 rounded-lg {cls.text_muted} {cls.hover_bg} \
                            hover:{cls.text_primary} transition-all duration-150 w-[30px]",
                    onclick: move |_| { let _ = nav.push(Route::Settings {}); },
                    span { class:"text-[20px]" , "⚙" }
                }
            }
        }
    }
}
