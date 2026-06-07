use dioxus::prelude::*;
use std::sync::Mutex;
use std::time::Duration;

use crate::components::log_panel::LogPanel;
use crate::components::sidebar::Sidebar;
use crate::components::theme_toggle::ThemeToggle;
use crate::components::toolbar::Toolbar;
use crate::gui_logger::LogBuffer;
use crate::state::app_state::AppState;
use crate::state::download_task::{compute_sha256, DownloadTask, TaskStatus};
use crate::state::log_entry::LogEntry;
use crate::state::theme::ThemeClasses;
use crate::Route;

/// Shared layout wrapping all pages: Toolbar + Sidebar + Outlet + LogPanel.
#[component]
pub fn Shell() -> Element {
    let cls = use_context::<Signal<ThemeClasses>>();
    let cls = cls();
    let state = use_context::<AppState>();
    let log_collapsed = use_signal(|| false);

    // Extract signals
    let mut tasks_sig = state.tasks;
    let mut sel_id = state.selected_id;
    let mut logs = state.logs;
    let filter = state.filter;
    let mut dirty = state.dirty;
    let log_buf = use_context::<LogBuffer>();

    // ── Periodic update tick ───────────────────────────────
    let mut tick = use_signal(|| 0u64);
    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            tick += 1;
        }
    });
    let _ = tick(); // force re-render on tick

    // ── Drain library logs ────────────────────────────────
    {
        let entries = crate::gui_logger::drain_buffer(&log_buf);
        if !entries.is_empty() {
            let mut log_list = logs.write();
            for e in entries {
                log_list.push(e);
            }
            // Keep bounded
            if log_list.len() > 5000 {
                log_list.drain(0..1000);
            }
        }
    }

    // ── Update tasks ───────────────────────────────────────
    let tasks = tasks_sig.read();
    let tasks_clone: Vec<DownloadTask> = tasks.iter().cloned().collect();
    drop(tasks);

    // Collect tasks that need SHA256 computation (outside the runtime lock)
    let mut sha256_queue: Vec<(u64, String)> = Vec::new();

    // Poll receivers and update progress
    for task in &tasks_clone {
        // Check SHA256 rx (separate call)
        let mut sha_hash: Option<String> = None;
        DownloadTask::with_runtime_id(task.id, |rt| {
            // Use as_ref().and_then() to drop MutexGuard before assigning rt.sha256_rx
            let hash = rt
                .sha256_rx
                .as_ref()
                .and_then(|rx| rx.lock().unwrap().try_recv().ok());
            if let Some(h) = hash {
                sha_hash = Some(h);
                rt.sha256_rx = None;
            }
        });
        if let Some(hash) = sha_hash {
            let mut tlist = tasks_sig.write();
            if let Some(t) = tlist.iter_mut().find(|t| t.id == task.id) {
                t.sha256 = Some(hash.clone());
                dirty.set(true);
            }
            logs.write()
                .push(LogEntry::app(format!("SHA256: {}", hash)));
        }

        // Check download receiver + poll progress
        DownloadTask::with_runtime_id(task.id, |rt| {
            // Extract result first so MutexGuard is dropped before assigning rt.receiver
            let recv_result = rt
                .receiver
                .as_ref()
                .and_then(|rx| rx.lock().unwrap().try_recv().ok());

            if let Some(result) = recv_result {
                match result {
                    Ok(df) => {
                        let real = df.get_real_file_path();
                        let fname = crate::state::download_task::extract_filename(&real);
                        let size = df.size();
                        let mut tlist = tasks_sig.write();
                        if let Some(t) = tlist.iter_mut().find(|t| t.id == task.id) {
                            t.filename = fname.clone();
                            t.file_path = real;
                            t.file_size = size;
                            t.status = TaskStatus::Downloading;
                        }
                        dirty.set(true);
                        logs.write()
                            .push(LogEntry::app(format!("Downloading: {}", fname)));
                        rt.download = Some(df);
                    }
                    Err(e) => {
                        let mut tlist = tasks_sig.write();
                        if let Some(t) = tlist.iter_mut().find(|t| t.id == task.id) {
                            t.status = TaskStatus::Error;
                            t.error_msg = Some(e.to_string());
                        }
                        dirty.set(true);
                        logs.write()
                            .push(LogEntry::app_error(format!("Failed: {}", e)));
                    }
                }
                rt.receiver = None;
            }

            // Poll active download
            if let Some(ref df) = rt.download {
                let status = df.get_status();
                let mut tlist = tasks_sig.write();
                if let Some(t) = tlist.iter_mut().find(|t| t.id == task.id) {
                    t.downloaded = status.get_down_size();
                    if df.size() > 0 {
                        t.file_size = df.size();
                    }
                    t.speed = status.get_byte_sec();
                    t.progress = status.get_percent_complete();
                    if matches!(t.status, TaskStatus::Starting | TaskStatus::Downloading) {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        t.elapsed = Duration::from_millis(now_ms - t.start_time_ms);
                    }

                    if status.is_finish()
                        && t.status != TaskStatus::Completed
                        && t.status != TaskStatus::Error
                    {
                        if status.is_error() {
                            t.status = TaskStatus::Error;
                            t.error_msg = status.get_error().map(|e| e.to_string());
                            dirty.set(true);
                            logs.write().push(LogEntry::app_error(format!(
                                "Error: {}",
                                t.error_msg.as_deref().unwrap_or("unknown")
                            )));
                        } else {
                            t.status = TaskStatus::Completed;
                            t.progress = 100.0;
                            t.downloaded = t.file_size;
                            t.speed = 0;
                            dirty.set(true);
                            t.file_path = df.get_real_file_path();
                            logs.write()
                                .push(LogEntry::app(format!("Completed: {}", t.file_path)));

                            // Queue SHA256 computation (must be done OUTSIDE the runtime lock)
                            if t.sha256.is_none() {
                                sha256_queue.push((t.id, t.file_path.clone()));
                                logs.write().push(LogEntry::app("Computing SHA256..."));
                            }
                        }
                    }
                }
            }
        });
    }

    // Process SHA256 queue (OUTSIDE the runtime lock to avoid deadlock)
    for (tid, file_path) in sha256_queue {
        let (tx, rx) = std::sync::mpsc::channel();
        let fp = file_path.clone();
        std::thread::spawn(move || {
            if let Ok(hash) = compute_sha256(&fp) {
                let _ = tx.send(hash);
            }
        });
        DownloadTask::with_runtime_id(tid, |rt_data| {
            rt_data.sha256_rx = Some(Mutex::new(rx));
        });
    }

    // ── Derived counts ─────────────────────────────────────
    let all_count = tasks_clone.len();
    let downloading_count = tasks_clone
        .iter()
        .filter(|t| {
            matches!(
                t.status,
                TaskStatus::Downloading | TaskStatus::Paused | TaskStatus::Starting
            )
        })
        .count();
    let completed_count = tasks_clone
        .iter()
        .filter(|t| matches!(t.status, TaskStatus::Completed | TaskStatus::Error))
        .count();
    let active_count = tasks_clone
        .iter()
        .filter(|t| t.status == TaskStatus::Downloading)
        .count();
    let total_speed: u64 = tasks_clone
        .iter()
        .filter(|t| t.status == TaskStatus::Downloading)
        .map(|t| t.speed)
        .sum();

    let sel_status =
        sel_id().and_then(|id| tasks_clone.iter().find(|t| t.id == id).map(|t| t.status));
    let can_pause = sel_status == Some(TaskStatus::Downloading);
    let can_resume = sel_status == Some(TaskStatus::Paused);

    // ── Action handlers ────────────────────────────────────
    let handle_pause = move |_| {
        if let Some(id) = sel_id() {
            let mut tlist = tasks_sig.write();
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
            logs.write()
                .push(LogEntry::app(format!("Paused task #{}", id)));
            dirty.set(true);
        }
    };

    let handle_resume = move |_| {
        if let Some(id) = sel_id() {
            let task = tasks_sig.read().iter().find(|t| t.id == id).cloned();
            if let Some(t) = task {
                let mut tlist = tasks_sig.write();
                if let Some(tm) = tlist.iter_mut().find(|t2| t2.id == id) {
                    tm.status = TaskStatus::Downloading;
                }
                drop(tlist);

                DownloadTask::with_runtime_id(id, |rt| {
                    if let Some(ref df) = rt.download {
                        df.restart();
                    } else {
                        let (tx, rx) = std::sync::mpsc::channel();
                        rt.receiver = Some(Mutex::new(rx));
                        let u = t.url.clone();
                        let s = t.save_dir.clone();
                        let tc = t.task_count;
                        let ck = t.cookies.clone();
                        crate::rt().spawn(async move {
                            let result = download_lib::DownloadFile::start_download(
                                u,
                                std::path::PathBuf::from(s),
                                tc,
                                1024 * 1024,
                                None,
                                ck,
                            )
                            .await;
                            let _ = tx.send(result);
                        });
                    }
                });
                logs.write()
                    .push(LogEntry::app(format!("Resumed task #{}", id)));
                dirty.set(true);
            }
        }
    };

    let handle_delete = move |_| {
        if let Some(id) = sel_id() {
            DownloadTask::with_runtime_id(id, |rt| {
                if let Some(ref df) = rt.download {
                    df.suspend();
                }
            });
            DownloadTask::remove_runtime(id);
            tasks_sig.write().retain(|t| t.id != id);
            sel_id.set(None);
            logs.write()
                .push(LogEntry::app(format!("Deleted task #{}", id)));
            dirty.set(true);
        }
    };

    // ── Auto-save dirty tasks ───────────────────────────────
    if dirty() {
        debug!("saved tasks");
        let task_list = tasks_sig.read();
        DownloadTask::save_all(&task_list)?;
        dirty.set(false);
    }

    rsx! {
        div { class: "flex flex-col h-screen {cls.page_bg} overflow-hidden",

            Toolbar {
                active_count,
                total_speed,
                selected_id: sel_id(),
                can_pause,
                can_resume,
                on_pause: handle_pause,
                on_resume: handle_resume,
                on_delete: handle_delete,
                theme_toggle: rsx! { ThemeToggle {} },
            }

            div { class: "flex flex-1 min-h-0",
                Sidebar { filter, all_count, downloading_count, completed_count }
                div { class: "flex-1 flex flex-col min-w-0 {cls.page_bg}",
                    Outlet::<Route> {}
                }
            }

            LogPanel { logs, collapsed: log_collapsed }
        }
    }
}
