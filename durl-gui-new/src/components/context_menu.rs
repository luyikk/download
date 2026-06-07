use dioxus::prelude::*;
use std::sync::Mutex;

use crate::state::app_state::AppState;
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

    let id = task.id;
    let file_path = task.file_path.clone();
    let url = task.url.clone();
    let save_dir = task.save_dir.clone();
    let task_count = task.task_count;
    let cookies = task.cookies.clone();

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
                            let id = id;
                            let file_path = file_path.clone();
                            let url = url.clone();
                            let save_dir = save_dir.clone();
                            let task_count = task_count;
                            let cookies = cookies.clone();

                            rsx! {
                                button {
                                    class: "w-full text-left px-3 py-1.5 text-sm \
                                            {cls.text_secondary} hover:{cls.text_primary} \
                                            hover:bg-[#6C5CE7]/10 transition-colors \
                                            disabled:opacity-30 disabled:pointer-events-none",
                                    disabled: !enabled,
                                    onclick: move |_| {
                                        handle_action(
                                            &key, id, &file_path, &url, &save_dir, task_count, &cookies,
                                            &mut state,
                                        );
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

fn handle_action(
    key: &str,
    id: u64,
    file_path: &str,
    url: &str,
    save_dir: &str,
    task_count: u64,
    cookies: &Option<String>,
    state: &mut AppState,
) {
    match key {
        "copy_url" => {
            (state.context_menu).set(None);
        }
        "pause" => {
            let mut tlist = state.tasks.write();
            if let Some(t) = tlist.iter_mut().find(|t| t.id == id) {
                t.status = TaskStatus::Paused;
                t.speed = 0;
            }
            drop(tlist);
            DownloadTask::with_runtime_id(id, |rt| {
                if let Some(ref df) = rt.download {
                    df.suspend();
                }
            });
            (state.logs)
                .write()
                .push(format!("[{}]  Paused task #{}", now_str(), id));
            (state.context_menu).set(None);
        }
        "resume" => {
            let mut tlist = state.tasks.write();
            if let Some(t) = tlist.iter_mut().find(|t| t.id == id) {
                t.status = TaskStatus::Downloading;
            }
            drop(tlist);
            DownloadTask::with_runtime_id(id, |rt| {
                if let Some(ref df) = rt.download {
                    df.restart();
                } else {
                    let (tx, rx) = std::sync::mpsc::channel();

                    let u = url.to_string();
                    let s = save_dir.to_string();
                    let ck = cookies.clone();
                    crate::rt().spawn(async move {
                        let result = download_lib::DownloadFile::start_download(
                            u,
                            std::path::PathBuf::from(s),
                            task_count,
                            1024 * 1024,
                            None,
                            ck,
                        )
                        .await;
                        let _ = tx.send(result);
                    });
                }
            });
            (state.logs)
                .write()
                .push(format!("[{}]  Resumed task #{}", now_str(), id));
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
            (state.context_menu).set(None);
        }
        "delete" => {
            DownloadTask::with_runtime_id(id, |rt| {
                if let Some(ref df) = rt.download {
                    df.suspend();
                }
            });
            DownloadTask::remove_runtime(id);
            state.tasks.write().retain(|t| t.id != id);
            (state.selected_id).set(None);
            (state.logs)
                .write()
                .push(format!("[{}]  Deleted task #{}", now_str(), id));
            (state.context_menu).set(None);
        }
        _ => {}
    }
}

fn now_str() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60
    )
}
