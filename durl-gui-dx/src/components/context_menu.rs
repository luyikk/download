use dioxus::prelude::*;

use crate::state::app_state::{
    AppState, HandleDeleteType, HandlePauseType, HandleReDownloadType, HandleResumeType,
};
use crate::state::download_task::{DownloadTask, TaskStatus};
use crate::state::i18n::LangStrings;
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
    let tasks = state.tasks.read();
    let task = tasks.iter().find(|t| t.id == task_id).cloned();
    drop(tasks);
    if task.is_none() {
        (state.context_menu).set(None);
        return rsx! {};
    }
    let task = task.unwrap();

    let close = move |_| {
        (state.context_menu).set(None);
    };

    // Build menu items as a Vec<(label, action_key)> so we can iterate
    let mut items: Vec<(String, String, bool, bool)> = Vec::new(); // (label, key, enabled, is_sep)
    items.push((
        lang.get("context_menu.copy_url").into(),
        "copy_url".into(),
        true,
        false,
    ));
    if task.sha256.is_some() {
        items.push((
            lang.get("context_menu.copy_sha256").into(),
            "copy_sha256".into(),
            true,
            false,
        ));
    }
    items.push(("".into(), "".into(), true, true));

    if task.status == TaskStatus::Downloading {
        items.push((
            lang.get("context_menu.pause").into(),
            "pause".into(),
            true,
            false,
        ));
    }
    if task.status == TaskStatus::Paused {
        items.push((
            lang.get("context_menu.resume").into(),
            "resume".into(),
            true,
            false,
        ));
    }
    if task.status == TaskStatus::Completed {
        items.push((
            lang.get("context_menu.open_file").into(),
            "open_file".into(),
            true,
            false,
        ));
        items.push((
            lang.get("context_menu.open_dir").into(),
            "open_dir".into(),
            true,
            false,
        ));
        items.push((
            lang.get("context_menu.redownload").into(),
            "redownload".into(),
            true,
            false,
        ));
    }
    items.push(("".into(), "".into(), true, true));
    items.push((
        lang.get("context_menu.delete").into(),
        "delete".into(),
        true,
        false,
    ));

    rsx! {
        div {
            class: "fixed inset-0 z-[60]",
            onclick: close,
            oncontextmenu: move |e| {
                e.stop_propagation();
                (state.context_menu).set(None);
            },
            div {
                class: "fixed z-[61] min-w-[200px] py-1.5 rounded-lg {cls.card_bg} \
                        border {cls.border} shadow-xl shadow-black/30",
                style: "left: {x}px; top: {y}px;",
                onclick: |e| e.stop_propagation(),
                oncontextmenu: |e| e.stop_propagation(),

                for (label, key, enabled, is_sep) in items.iter() {
                    if *is_sep {
                        div { class: "my-1 border-t {cls.border}" }
                    } else {
                        {
                            let label = label.clone();
                            let key = key.clone();
                            let enabled = *enabled;
                            let task = task.clone();

                            rsx! {
                                button {
                                    class: "w-full text-left px-3 py-1.5 text-sm \
                                            {cls.text_secondary} hover:{cls.text_primary} \
                                            hover:bg-[#6C5CE7]/10 transition-colors \
                                            disabled:opacity-30 disabled:pointer-events-none translate-x-[5px]",
                                    disabled: !enabled,
                                    onclick: move |_| {
                                        handle_action(&key, &task, &mut state);
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_action(key: &str, task: &DownloadTask, state: &mut AppState) {
    let id = task.id;
    let file_path = &task.file_path;
    let url = &task.url;
    let sha256 = &task.sha256;

    match key {
        "copy_url" => {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(url);
            }
            log::info!("Copied URL for task #{id}");
            (state.context_menu).set(None);
        }
        "copy_sha256" => {
            if let Some(ref hash) = sha256 {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(hash);
                }
                log::info!("Copied SHA256 for task #{}", id);
            }
            (state.context_menu).set(None);
        }
        "pause" => {
            consume_context::<HandlePauseType>().call(id.into());
            (state.context_menu).set(None);
        }
        "resume" => {
            consume_context::<HandleResumeType>().call(id.into());
            (state.context_menu).set(None);
        }
        "open_file" => {
            let _ = open::that(file_path);
            (state.context_menu).set(None);
        }
        "open_dir" => {
            if let Some(parent) = std::path::Path::new(file_path).parent() {
                let _ = open::that(parent);
            }
            (state.context_menu).set(None);
        }
        "redownload" => {
            consume_context::<HandleReDownloadType>().call(id.into());
            (state.context_menu).set(None);
        }
        "delete" => {
            consume_context::<HandleDeleteType>().call(id.into()); // Ensure task is paused before deletion
            log::info!("Deleted task #{}", id);
            (state.selected_id).set(None);
            (state.context_menu).set(None);
        }
        _ => {}
    }
}
