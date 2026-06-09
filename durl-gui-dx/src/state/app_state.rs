use crate::browser_server::BrowserDownloadReq;
use crate::gui_logger::LogBuffer;
use crate::state::download_task::{
    compute_sha256, extract_filename, DownloadTask, Filter, TaskStatus,
};
use crate::state::log_entry::LogEntry;
use dashmap::DashMap;
use derive_more::From;
use dioxus::prelude::*;
use download_lib::DownloadFile;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Runtime-only data that can't be cloned / put in signals.
/// Wraps `mpsc::Receiver` in `Mutex` so that `RuntimeData` is `Sync` (required by `DashMap`).
pub struct RuntimeData {
    pub download: anyhow::Result<DownloadFile, download_lib::DownloadError>,
}

/// Global storage for runtime-only task data, keyed by task id.
/// Uses `DashMap` for lock-free concurrent access — no global Mutex needed.
static RUNTIME: Lazy<DashMap<u64, RuntimeData>> = Lazy::new(DashMap::new);

/// Action type for starting a new download, triggered by the NewDownload dialog.
pub type NewDownLoadType = Action<(NewDownloadContext,), ()>;

/// Wrapper type for pausing a download, used in the HandlePauseType action.
#[derive(From)]
pub struct PauseType(u64);
/// Action type for pausing a download, triggered by the context menu on a task.
pub type HandlePauseType = Action<(PauseType,), ()>;

/// Wrapper type for resuming a download, used in the HandleResumeType action.
#[derive(From)]
pub struct ResumeType(u64);
/// Action type for resuming a paused download, triggered by the context menu on a task.
pub type HandleResumeType = Action<(ResumeType,), ()>;

/// Wrapper type for deleting a download, used in the HandleDeleteType action.
#[derive(From)]
pub struct DeleteType(u64);
/// Action type for deleting a download, triggered by the context menu on a task.
pub type HandleDeleteType = Action<(DeleteType,), ()>;

/// Wrapper type for re-downloading a task, used in the HandleReDownloadType action.
#[derive(From)]
pub struct ReDownloadType(u64);
/// Action type for re-downloading a task (delete + start new with same URL), triggered by the context menu on a task.
pub type HandleReDownloadType = Action<(ReDownloadType,), ()>;

/// Context data for starting a new download, passed from the NewDownload dialog to AppState::new_download().
#[derive(Debug, Clone)]
pub struct NewDownloadContext {
    pub url: String,
    pub save_path: PathBuf,
    pub task_count: u64,
    pub filename: Option<String>,
    pub cookies: Option<String>,
}

/// Shared application state, provided via context at the root.
#[derive(Clone, Copy)]
pub struct AppState {
    pub tasks: Signal<Vec<DownloadTask>>,
    pub filter: Signal<Filter>,
    pub selected_id: Signal<Option<u64>>,
    pub logs: Signal<Vec<LogEntry>>,
    pub show_new_dialog: Signal<bool>,
    /// Pre-fill data for the NewDownload dialog (from browser extension).
    pub browser_req: Signal<Option<BrowserDownloadReq>>,
    /// Browser extension install guide dialog.
    pub show_ext_install: Signal<bool>,
    /// Path to extracted extension folder.
    pub ext_install_path: Signal<String>,
    /// Which browser's extensions URL to show (chrome:// or edge://).
    pub ext_browser_url: Signal<String>,
    /// Right-click context menu: (task_id, screen_x, screen_y)
    pub context_menu: Signal<Option<(u64, f64, f64)>>,
    /// Dirty flag — set true when tasks change, triggers auto-save.
    pub dirty: Signal<bool>,
    /// Sha256 Update Queue
    pub sha256_queue: Signal<DashMap<u64, tokio::sync::oneshot::Receiver<(u64, String)>>>,
}

impl AppState {
    /// Update task statuses by polling the runtime data and download handles.
    pub async fn update() {
        let mut app_state = consume_context::<AppState>();

        for task in app_state.tasks.write().iter_mut() {
            if let Some(runtime) = RUNTIME.get(&task.id) {
                if task.status == TaskStatus::Starting {
                    match &runtime.download {
                        Ok(df) => {
                            let real = df.get_real_file_path();
                            let file_name = crate::state::download_task::extract_filename(&real);
                            let size = df.size();

                            task.filename = file_name.clone();
                            task.file_path = real;
                            task.file_size = size;
                            task.start_time_ms = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                                as u64;
                            task.status = TaskStatus::Downloading;
                            app_state.dirty.set(true);
                            log::info!("Downloading: {}", file_name);
                        }
                        Err(e) => {
                            task.status = TaskStatus::Error;
                            task.error_msg = Some(e.to_string());
                            app_state.dirty.set(true);
                            log::error!("Failed: {}", e);
                        }
                    }
                } else if task.status == TaskStatus::Downloading {
                    if let Ok(df) = &runtime.download {
                        let status = df.get_status();
                        task.downloaded = status.get_down_size();
                        if df.size() > 0 {
                            task.file_size = df.size();
                        }
                        task.speed = status.get_byte_sec();
                        task.progress = status.get_percent_complete();

                        let now_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        task.elapsed = Duration::from_millis(now_ms - task.start_time_ms);

                        if status.is_finish() {
                            if status.is_error() {
                                task.status = TaskStatus::Error;
                                task.error_msg = status.get_error().map(|e| e.to_string());
                                app_state.dirty.set(true);
                                log::error!(
                                    "Error: {}",
                                    task.error_msg.as_deref().unwrap_or("unknown")
                                );
                            } else {
                                task.status = TaskStatus::Completed;
                                task.progress = 100.0;
                                task.downloaded = task.file_size;
                                task.speed = 0;
                                app_state.dirty.set(true);
                                task.file_path = df.get_real_file_path();
                                log::info!("Completed: {}", task.file_path);

                                // Queue SHA256 computation (must be done OUTSIDE the runtime lock)
                                if task.sha256.is_none() {
                                    // Assuming there's a global queue for SHA256 tasks
                                    let file_path = df.get_real_file_path();
                                    let task_id = task.id;

                                    let (tx, rx) = tokio::sync::oneshot::channel();
                                    tokio::spawn(async move {
                                        if let Ok(hash) = compute_sha256(&file_path) {
                                            log::info!("SHA256: {}", hash);
                                            let _ = tx.send((task_id, hash));
                                        } else {
                                            log::error!("Failed to compute SHA256: {file_path}");
                                        }
                                    });

                                    app_state.sha256_queue.write().insert(task_id, rx);
                                    log::info!("Computing SHA256...");
                                }
                            }
                        }
                    }
                }
            }
        }

        let log_buf = use_context::<LogBuffer>();
        let entries = crate::gui_logger::drain_buffer(&log_buf);
        if !entries.is_empty() {
            let mut log_list = app_state.logs.write();
            for e in entries {
                log_list.push(e);
            }
            // Keep bounded
            if log_list.len() > 5000 {
                log_list.drain(0..1000);
            }
        }

        // ── Poll browser extension channel ────────────────────
        if let Some(req) = crate::try_recv_browser_req() {
            log::trace!(
                "Received download request from browser: url={}, cookies={:?}",
                req.url,
                req.cookies
            );
            app_state.browser_req.set(Some(req));
            app_state.show_new_dialog.set(true);
        }

        let completed_hashes = app_state
            .sha256_queue
            .read()
            .iter_mut()
            .filter_map(|mut s| s.value_mut().try_recv().ok())
            .collect::<Vec<_>>();

        if !completed_hashes.is_empty() {
            let mut tasks_lock = app_state.tasks.write();
            let queue_lock = app_state.sha256_queue.read();
            for (task_id, hash) in completed_hashes.iter() {
                if let Some(task) = tasks_lock.iter_mut().find(|t| t.id == *task_id) {
                    task.sha256 = Some(hash.clone());
                }
                queue_lock.remove(task_id);
            }

            app_state.dirty.set(true);
        }
    }

    /// Start a new download task with the given context (from NewDownload dialog).
    pub async fn new_download(data: NewDownloadContext) {
        let mut app_state = consume_context::<AppState>();
        let id = {
            let tasks = app_state.tasks.read();
            tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1
        };

        let url = data.url;
        if url.is_empty() {
            return;
        }
        let save_path = data.save_path;
        let task_count: u64 = data.task_count.max(1).clamp(1, 64);
        let file_name = data.filename;
        let cookies = data.cookies;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let display_name = file_name.clone().unwrap_or_else(|| extract_filename(&url));

        log::info!("Starting: {}", display_name);

        let task = DownloadTask {
            id,
            url: url.clone(),
            filename: display_name,
            file_path: String::new(),
            save_dir: save_path.display().to_string(),
            file_size: 0,
            downloaded: 0,
            speed: 0,
            progress: 0.0,
            status: TaskStatus::Starting,
            error_msg: None,
            elapsed: Duration::ZERO,
            start_time_ms: now_ms,
            task_count,
            cookies: cookies.clone(),
            sha256: None,
        };

        app_state.tasks.write().push(task);

        let handle = DownloadFile::start_download(
            url,
            save_path.clone(),
            task_count,
            1024 * 1024,
            file_name,
            cookies,
        )
        .await;

        RUNTIME
            .entry(id)
            .insert_entry(RuntimeData { download: handle });

        app_state.browser_req.set(None);
    }

    /// Pause the download task with the given id.
    pub async fn handle_pause(id: PauseType) {
        let id = id.0;
        let mut app_state = consume_context::<AppState>();

        if let Some(t) = app_state.tasks.write().iter_mut().find(|t| t.id == id) {
            t.status = TaskStatus::Paused;
            t.speed = 0;
        }

        if let Some(rt) = RUNTIME.get(&id) {
            if let Ok(ref df) = rt.download {
                df.suspend();
            }
        }

        log::info!("Paused task #{}", id);

        app_state.dirty.set(true);
    }

    /// Resume the download task with the given id.
    pub async fn handle_resume(id: ResumeType) {
        let id = id.0;
        let mut app_state = consume_context::<AppState>();
        let task = {
            let mut tasks = app_state.tasks.write();
            if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
                t.status = TaskStatus::Downloading;
                Some(t.clone())
            } else {
                None
            }
        };

        if let Some(t) = task {
            log::info!("Resume task #{}", id);

            if let Some(rt) = RUNTIME.get(&id) {
                if let Ok(ref df) = rt.download {
                    df.restart()
                }
            } else {
                let data = RuntimeData {
                    download: DownloadFile::start_download(
                        &t.url,
                        t.save_dir.into(),
                        t.task_count,
                        1024 * 1024,
                        None,
                        t.cookies,
                    )
                    .await,
                };
                RUNTIME.insert(id, data);
            }

            app_state.dirty.set(true);
        }
    }

    /// Delete the download task with the given id.
    pub async fn handle_delete(id: DeleteType) {
        let id = id.0;
        let mut app_state = consume_context::<AppState>();
        if let Some((_, task)) = RUNTIME.remove(&id) {
            if let Ok(ref df) = task.download {
                df.suspend();
            }
        }
        app_state.tasks.write().retain(|t| t.id != id);

        log::info!("Delete task #{}", id);
        app_state.dirty.set(true);
    }

    /// Redownload the task with the given id (delete + start new with same URL).
    pub async fn handle_redownload(id: ReDownloadType) {
        let id = id.0;
        let mut app_state = consume_context::<AppState>();
        let task = {
            let mut tasks = app_state.tasks.write();
            if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
                t.status = TaskStatus::Starting;
                t.speed = 0;
                t.downloaded = 0;
                t.progress = 0.0;
                t.file_size = 0;
                t.error_msg = None;
                t.sha256 = None;
                Some(t.clone())
            } else {
                None
            }
        };

        if let Some(t) = task {
            if !t.file_path.is_empty() {
                let _ = std::fs::remove_file(t.file_path);
            }

            let custom_name = if t.filename.is_empty() {
                None
            } else {
                Some(t.filename)
            };

            if let Some((_, task)) = RUNTIME.remove(&id) {
                if let Ok(ref df) = task.download {
                    df.suspend();
                }
            }

            let handle = DownloadFile::start_download(
                t.url,
                t.save_dir.into(),
                t.task_count,
                1024 * 1024,
                custom_name,
                t.cookies,
            )
            .await;

            RUNTIME
                .entry(id)
                .insert_entry(RuntimeData { download: handle });

            log::info!("Re-downloading task #{}", id);
        }
    }
}
