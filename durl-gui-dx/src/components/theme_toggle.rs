use dioxus::prelude::*;

use crate::state::config::UserConfig;
use crate::state::theme::{ThemeClasses, DARK, LIGHT};

/// Theme toggle button — switches between dark and light, persists to config.
#[component]
pub fn ThemeToggle() -> Element {
    let mut theme = use_context::<Signal<ThemeClasses>>();
    let mut cfg = use_context::<Signal<UserConfig>>();

    let is_dark = theme() == DARK;
    let icon = if is_dark { "☀" } else { "🌙" };
    let tooltip = if is_dark {
        "Switch to light mode"
    } else {
        "Switch to dark mode"
    };

    rsx! {
        button {
            class: "p-3 rounded-lg text-slate-400 {theme().hover_bg} hover:text-slate-200 \
                    transition-all duration-150 w-[30px]",
            title: tooltip,
            onclick: move |_| {
                if theme() == DARK {
                    theme.set(LIGHT);
                    cfg.write().theme = "light".into();
                } else {
                    theme.set(DARK);
                    cfg.write().theme = "dark".into();
                }
                cfg.read().save();
            },
            span { class: "text-[20px]", "{icon}" }
        }
    }
}
