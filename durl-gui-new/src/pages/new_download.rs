use dioxus::prelude::*;

use crate::state::app_state::AppState;
use crate::state::config::UserConfig;
use crate::state::i18n::LangStrings;
use crate::state::theme::ThemeClasses;

/// New download dialog — modal overlay.
#[component]
pub fn NewDownload() -> Element {
    let cls = use_context::<Signal<ThemeClasses>>();
    let cls = cls();
    let lang_sig = use_context::<Signal<LangStrings>>();
    let lang = lang_sig();
    let cfg = use_context::<Signal<UserConfig>>();
    let mut state = use_context::<AppState>();

    // ── Form state (pre-filled from config) ──────────────────
    let mut url = use_signal(String::new);
    let mut save_path = use_signal(|| cfg().default_save_path.clone());
    let mut filename = use_signal(String::new);
    let mut task_count = use_signal(|| cfg().default_task_count.to_string());

    // ── i18n ─────────────────────────────────────────────────
    let title = lang.get("dialog_new.title").to_string();
    let lbl_url = lang.get("dialog_new.url_label").to_string();
    let ph_url = lang.get("dialog_new.url_placeholder").to_string();
    let lbl_save = lang.get("dialog_new.save_dir").to_string();
    let lbl_filename = lang.get("dialog_new.filename_label").to_string();
    let ph_filename = lang.get("dialog_new.filename_placeholder").to_string();
    let lbl_threads = lang.get("dialog_new.concurrency_label").to_string();
    let lbl_cancel = lang.get("dialog_new.cancel").to_string();
    let lbl_start = lang.get("dialog_new.start").to_string();
    let lbl_browse = lang.get("dialog_new.browse").to_string();

    // ── Close/Cancel handler ─────────────────────────────────
    let close = move |_| {
        state.show_new_dialog.set(false);
    };

    // ── Start handler (placeholder) ──────────────────────────
    let start_download = move |_| {
        // TODO: integrate with download-lib
        state.show_new_dialog.set(false);
    };

    let can_start = !url().trim().is_empty();

    rsx! {
        // Full-screen backdrop
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center \
                    bg-black/60 backdrop-blur-sm",
            onclick: close,

            // Dialog card — stop click propagation
            div {
                class: "w-[780px] max-h-[90vh] rounded {cls.card_bg} border {cls.border} \
                        shadow-2xl shadow-black/40 flex flex-col overflow-hidden \
                        animate-slide-up",
                onclick: |e| e.stop_propagation(),

                // ── Header ────────────────────────────────────
                div { class: "flex items-center justify-between px-8 py-5 \
                              border-b {cls.border} shrink-0",
                    h2 { class: "text-lg font-semibold {cls.text_primary}",
                        "{title}"
                    }
                    button {
                        class: "p-1 rounded-lg {cls.text_muted} hover:{cls.text_primary} \
                                transition-colors",
                        onclick: close,
                        span { class: "text-xl", "✕" }
                    }
                }

                // ── Form body ─────────────────────────────────
                div { class: "flex-1 overflow-y-auto px-8 py-6 space-y-6",

                    // URL
                    div { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted}",
                            "{lbl_url}"
                        }
                        input {
                            class: "w-full px-4 py-2.5 rounded-lg {cls.input_bg} border {cls.input_border} \
                                    {cls.text_primary} text-base outline-none transition-colors \
                                    focus:border-[#6C5CE7]/50",
                            r#type: "text",
                            placeholder: "{ph_url}",
                            value: "{url}",
                            oninput: move |ev| url.set(ev.value()),
                        }
                    }

                    // Save directory
                    div { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted}",
                            "{lbl_save}"
                        }
                        div { class: "flex gap-2",
                            input {
                                class: "flex-1 px-4 py-2.5 rounded-lg {cls.input_bg} border {cls.input_border} \
                                        {cls.text_primary} text-base outline-none transition-colors \
                                        focus:border-[#6C5CE7]/50",
                                r#type: "text",
                                value: "{save_path}",
                                oninput: move |ev| save_path.set(ev.value()),
                            }
                            button {
                                class: "px-4 py-2.5 rounded-lg border {cls.border} {cls.text_secondary} \
                                        {cls.hover_bg} hover:{cls.text_primary} text-base \
                                        transition-colors shrink-0",
                                onclick: move |_| {
                                    if let Some(dir) = rfd::FileDialog::new()
                                        .set_directory(&save_path())
                                        .pick_folder()
                                    {
                                        save_path.set(dir.display().to_string());
                                    }
                                },
                                "{lbl_browse}"
                            }
                        }
                    }

                    // Filename
                    div { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted}",
                            "{lbl_filename}"
                        }
                        input {
                            class: "w-full px-4 py-2.5 rounded-lg {cls.input_bg} border {cls.input_border} \
                                    {cls.text_primary} text-base outline-none transition-colors \
                                    focus:border-[#6C5CE7]/50",
                            r#type: "text",
                            placeholder: "{ph_filename}",
                            value: "{filename}",
                            oninput: move |ev| filename.set(ev.value()),
                        }
                    }

                    // Concurrency
                    div { class: "space-y-2",
                        label { class: "block text-sm font-semibold {cls.text_muted}",
                            "{lbl_threads}"
                        }
                        input {
                            class: "w-24 px-4 py-2.5 rounded-lg {cls.input_bg} border {cls.input_border} \
                                    {cls.text_primary} text-base outline-none transition-colors \
                                    focus:border-[#6C5CE7]/50",
                            r#type: "number",
                            value: "{task_count}",
                            min: "1",
                            max: "64",
                            oninput: move |ev| task_count.set(ev.value()),
                        }
                    }
                }

                // ── Footer ────────────────────────────────────
                div { class: "flex items-center justify-end gap-3 px-8 py-5 \
                              border-t {cls.border} shrink-0",
                    button {
                        class: "px-5 py-2.5 rounded-lg text-base font-medium \
                                border {cls.border} {cls.text_secondary} \
                                {cls.hover_bg} hover:{cls.text_primary} \
                                transition-all duration-150",
                        onclick: close,
                        "{lbl_cancel}"
                    }
                    button {
                        class: "px-6 py-2.5 rounded-lg text-base font-medium \
                                bg-[#6C5CE7] text-white \
                                hover:bg-[#7C6CF7] active:bg-[#5C4CD7] \
                                transition-all duration-150 \
                                disabled:opacity-40 disabled:pointer-events-none",
                        disabled: !can_start,
                        onclick: start_download,
                        "{lbl_start}"
                    }
                }
            }
        }
    }
}
