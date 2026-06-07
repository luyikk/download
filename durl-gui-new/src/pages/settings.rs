use dioxus::prelude::*;

use crate::state::config::UserConfig;
use crate::state::i18n::LangStrings;
use crate::state::log_entry::LOG_LEVELS;
use crate::state::theme::{ThemeClasses, DARK};
use crate::Route;

/// Settings page with form layout, persisted to user.toml.
#[component]
pub fn Settings() -> Element {
    let cls = use_context::<Signal<ThemeClasses>>();
    let cls = cls();
    let lang_ctx = use_context::<Signal<LangStrings>>();
    let lang = lang_ctx();
    let mut cfg = use_context::<Signal<UserConfig>>();
    let mut theme = use_context::<Signal<ThemeClasses>>();
    let mut lang_sig = use_context::<Signal<LangStrings>>();
    let nav = navigator();

    // ── Local copies for form editing ────────────────────────
    let mut save_path = use_signal(|| cfg().default_save_path.clone());
    let mut task_count = use_signal(|| cfg().default_task_count.to_string());
    let mut log_level = use_signal(|| cfg().log_level.clone());
    let current_lang = lang.lang_id.clone();
    let current_theme = cfg().theme.clone();

    // ── Save handler ─────────────────────────────────────────
    let lang_for_save = lang.clone();
    let save_config = move |_| {
        cfg.write().default_save_path = save_path();
        cfg.write().language = lang_for_save.lang_id.clone();
        cfg.write().theme = if theme() == DARK {
            "dark".into()
        } else {
            "light".into()
        };
        if let Ok(n) = task_count().parse::<u64>() {
            cfg.write().default_task_count = n;
        }
        cfg.write().log_level = log_level();
        // Apply log level change at runtime
        crate::gui_logger::set_log_level(cfg.read().log_level_filter());
        cfg.read().save();
    };

    // ── Available options ────────────────────────────────────
    let avail_langs = LangStrings::available();
    let avail_themes: &[(&str, &str)] = &[("dark", "Dark"), ("light", "Light")];

    // ── Pre-compute all i18n strings ─────────────────────────
    let lbl_back = lang.get("page_settings.back").to_string();
    let lbl_title = lang.get("page_settings.title").to_string();
    let lbl_save_dir = lang.get("page_settings.save_dir").to_string();
    let lbl_placeholder = lang.get("page_settings.save_dir_placeholder").to_string();
    let lbl_browse = lang.get("page_settings.browse").to_string();
    let lbl_concurrency = lang.get("page_settings.concurrency").to_string();
    let lbl_log_level = lang.get("page_settings.log_level").to_string();
    let lbl_language = lang.get("page_settings.language_label").to_string();
    let lbl_theme = lang.get("page_settings.theme_label").to_string();
    let lbl_save = lang.get("page_settings.save").to_string();

    rsx! {
        div { class: "flex flex-col h-full",

            // Header
            div { class: "flex items-center justify-between \
                          {cls.border} border-b shrink-0",
                div { class: "flex items-center gap-3",
                    button {
                        class: "{cls.text_muted} hover:{cls.text_primary} transition-colors text-sm",
                        onclick: move |_| { let _ = nav.push(Route::Downloads {}); },
                        span { "{lbl_back}" }
                    }
                    h1 { class: "text-sm font-semibold {cls.text_primary}", "{lbl_title}" }
                }
            }

            // Settings form
            div { class: "flex-1 overflow-y-auto",
                div { class: "max-w-lg mx-auto !p-2 space-y-8",

                    // ── Save Directory ───────────────────────
                    section { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted} uppercase tracking-wider",
                            "{lbl_save_dir}"
                        }
                        div { class: "flex gap-2",
                            input {
                                class: "flex-1 px-3 py-2 rounded-lg {cls.input_bg} border {cls.input_border} \
                                        {cls.text_primary} text-sm outline-none transition-colors \
                                        focus:border-[#6C5CE7]/50",
                                r#type: "text",
                                placeholder: "{lbl_placeholder}",
                                value: "{save_path}",
                                oninput: move |ev| save_path.set(ev.value()),
                            }
                            button {
                                class: "px-4 py-2 rounded-lg border {cls.border} {cls.text_secondary} \
                                        {cls.hover_bg} hover:{cls.text_primary} text-sm transition-colors  \
                                        shrink-0 min-w-[60px]",
                                onclick: move |_| {
                                    if let Some(dir) = rfd::FileDialog::new()
                                        .set_directory(save_path())
                                        .pick_folder()
                                    {
                                        save_path.set(dir.display().to_string());
                                    }
                                },
                                "{lbl_browse}"
                            }
                        }
                    }

                    // ── Concurrency ──────────────────────────
                    section { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted} uppercase tracking-wider",
                            "{lbl_concurrency}"
                        }
                        input {
                            class: "w-24 px-3 py-2 rounded-lg {cls.input_bg} border {cls.input_border} \
                                    {cls.text_primary} text-sm outline-none transition-colors \
                                    focus:border-[#6C5CE7]/50",
                            r#type: "number",
                            value: "{task_count}",
                            min: "1",
                            max: "64",
                            oninput: move |ev| task_count.set(ev.value()),
                        }
                        p { class: "text-xs {cls.text_muted}", "Range: 1 – 64" }
                    }

                    // ── Log Level ───────────────────────────
                    section { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted} uppercase tracking-wider",
                            "{lbl_log_level}"
                        }
                        select {
                            class: "w-48 px-3 py-2 rounded-lg {cls.input_bg} border {cls.input_border} \
                                    {cls.text_primary} text-sm outline-none transition-colors \
                                    focus:border-[#6C5CE7]/50 cursor-pointer",
                            onchange: move |ev| log_level.set(ev.value()),
                            for level in LOG_LEVELS.iter() {
                                option {
                                    value: "{level}",
                                    selected: log_level() == *level,
                                    "{level}"
                                }
                            }
                        }
                    }

                    // ── Language ─────────────────────────────
                    section { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted} uppercase tracking-wider",
                            "{lbl_language}"
                        }
                        select {
                            class: "w-48 px-3 py-2 rounded-lg {cls.input_bg} border {cls.input_border} \
                                    {cls.text_primary} text-sm outline-none transition-colors \
                                    focus:border-[#6C5CE7]/50 cursor-pointer",
                            onchange: move |ev| {
                                let id = ev.value();
                                lang_sig.set(LangStrings::load(&id));
                            },
                            for (id, name) in avail_langs.iter() {
                                option {
                                    value: "{id}",
                                    selected: current_lang == *id,
                                    "{name}"
                                }
                            }
                        }
                    }

                    // ── Theme ────────────────────────────────
                    section { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted} uppercase tracking-wider",
                            "{lbl_theme}"
                        }
                        div { class: "flex gap-3",
                            for (theme_id, theme_name) in avail_themes.iter() {
                                {
                                    let id = *theme_id;
                                    let is_current = current_theme == id;
                                    let active_class = if is_current {
                                        format!(
                                            "{} border-[#6C5CE7]/50 ring-1 ring-[#6C5CE7]/20",
                                            cls.active_item_bg,
                                        )
                                    } else {
                                        format!(
                                            "border-transparent {} {}",
                                            cls.hover_bg, cls.card_bg,
                                        )
                                    };
                                    rsx! {
                                        button {
                                            class: "px-4 py-2.5 rounded-lg border text-sm \
                                                    {cls.text_primary} transition-all duration-150 \
                                                    {active_class} min-w-[60px]",
                                            onclick: move |_| {
                                                if id == "light" {
                                                    theme.set(crate::state::theme::LIGHT);
                                                } else {
                                                    theme.set(DARK);
                                                }
                                                cfg.write().theme = id.to_string();
                                                cfg.read().save();
                                            },
                                            span { class: "mr-2",
                                                if id == "dark" { "🌙" } else { "☀" }
                                            }
                                            "{theme_name}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Save ────────────────────────────────
                    div { class: "pt-4 border-t {cls.border}",
                        button {
                            class: "px-6 py-2 rounded-lg text-sm font-medium \
                                    bg-[#6C5CE7] text-white \
                                    hover:bg-[#7C6CF7] active:bg-[#5C4CD7] \
                                    transition-all duration-150 min-w-[80px]",
                            onclick: save_config,
                            "{lbl_save}"
                        }
                    }
                }
            }
        }
    }
}
