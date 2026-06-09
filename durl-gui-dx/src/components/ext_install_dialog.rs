use dioxus::prelude::*;

use crate::state::app_state::AppState;
use crate::state::i18n::LangStrings;
use crate::state::theme::ThemeClasses;

/// Modal dialog guiding the user through browser extension installation.
#[component]
pub fn ExtInstallDialog() -> Element {
    let cls = use_context::<Signal<ThemeClasses>>()();
    let lang = use_context::<Signal<LangStrings>>()();

    let mut state = use_context::<AppState>();

    let ext_path = (state.ext_install_path)();
    let ext_browser_url = (state.ext_browser_url)();

    let title = lang.get("dialog_ext_install.title").to_string();
    let lbl_path = lang.get("dialog_ext_install.path_label").to_string();
    let lbl_copy_path = lang.get("dialog_ext_install.copy_path").to_string();
    let step1 = lang.get("dialog_ext_install.step1").to_string();
    let step2 = if ext_browser_url.starts_with("edge") {
        lang.get("dialog_ext_install.step2_edge").to_string()
    } else {
        lang.get("dialog_ext_install.step2").to_string()
    };
    let step3 = lang.get("dialog_ext_install.step3").to_string();
    let step4 = lang.get("dialog_ext_install.step4").to_string();
    let lbl_url_label = if ext_browser_url.starts_with("edge") {
        lang.get("dialog_ext_install.edge_url_label").to_string()
    } else {
        lang.get("dialog_ext_install.chrome_url_label").to_string()
    };
    let lbl_copy_url = lang.get("dialog_ext_install.copy_url").to_string();
    let lbl_close = lang.get("dialog_ext_install.close").to_string();

    let close = move |_| {
        state.show_ext_install.set(false);
    };

    let path_for_copy = ext_path.clone();
    let copy_path = move |_| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(&path_for_copy);
        }
    };

    let url_for_copy = ext_browser_url.clone();
    let copy_url = move |_| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(&url_for_copy);
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm",
            onclick: close,
            div {
                class: "!p-1 w-[560px] max-h-[90vh] rounded {cls.card_bg} border {cls.border} \
                        shadow-2xl shadow-black/40 flex flex-col overflow-hidden animate-slide-up",
                onclick: |e| e.stop_propagation(),

                // Header
                div { class: "flex items-center justify-between px-6 py-4 border-b {cls.border} shrink-0",
                    h2 { class: "text-lg font-semibold {cls.text_primary}", "{title}" }
                    button {
                        class: "p-1 rounded-sm {cls.text_muted} hover:{cls.text_primary} \
                                {cls.hover_bg} transition-colors min-w-7",
                        onclick: close,
                        span { class: "text-xl", "✕" }
                    }
                }

                // Body
                div { class: "flex-1 overflow-y-auto px-6 py-5 space-y-4",
                    // Extension folder path
                    div { class: "space-y-1.5",
                        label { class: "block text-sm font-semibold {cls.text_primary}",
                            "{lbl_path}"
                        }
                        div { class: "flex gap-2",
                            input {
                                class: "flex-1 px-3 py-2 rounded-lg {cls.input_bg} border \
                                        {cls.input_border} {cls.text_primary} text-sm outline-none \
                                        select-all",
                                r#type: "text",
                                value: "{ext_path}",
                                readonly: true,
                            }
                            button {
                                class: "px-3 py-2 rounded-lg text-sm font-medium \
                                        border {cls.border} {cls.text_secondary} \
                                        {cls.hover_bg} hover:{cls.text_primary} \
                                        transition-colors shrink-0",
                                onclick: copy_path,
                                "{lbl_copy_path}"
                            }
                        }
                    }

                    // Installation steps
                    div { class: "space-y-2",
                        p { class: "text-sm {cls.text_secondary}", "{step1}" }
                        p { class: "text-sm {cls.text_secondary}", "{step2}" }
                        p { class: "text-sm {cls.text_secondary}", "{step3}" }
                        p { class: "text-sm {cls.text_secondary}", "{step4}" }
                    }

                    // Browser extensions URL
                    div { class: "space-y-1.5 pt-2 border-t {cls.border}",
                        label { class: "block text-sm font-semibold {cls.text_primary}",
                            "{lbl_url_label}"
                        }
                        div { class: "flex gap-2",
                            input {
                                class: "flex-1 px-3 py-2 rounded-lg {cls.input_bg} border \
                                        {cls.input_border} {cls.text_primary} text-sm outline-none \
                                        select-all",
                                r#type: "text",
                                value: "{ext_browser_url}",
                                readonly: true,
                            }
                            button {
                                class: "px-3 py-2 rounded-lg text-sm font-medium \
                                        border {cls.border} {cls.text_secondary} \
                                        {cls.hover_bg} hover:{cls.text_primary} \
                                        transition-colors shrink-0",
                                onclick: copy_url,
                                "{lbl_copy_url}"
                            }
                        }
                    }
                }

                // Footer
                div { class: "flex items-center justify-end gap-3 px-6 py-4 border-t {cls.border} shrink-0",
                    button {
                        class: "!px-6 !mt-1 !py-0.5 rounded-lg text-sm font-medium \
                                bg-[#6C5CE7] text-white \
                                hover:bg-[#7C6CF7] active:bg-[#5C4CD7] \
                                transition-all duration-150 min-w-[80px]",
                        onclick: close,
                        "{lbl_close}"
                    }
                }
            }
        }
    }
}
