use crate::config::{UserConfig, LOG_LEVELS};
use crate::gui_logger::LogBuffer;
use crate::i18n::{available_languages, LangStrings};
use download_lib::DownloadFile;
use eframe::egui::{self, Color32, RichText, Rounding, Vec2};
use humansize::{format_size, BINARY};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ── Column widths ────────────────────────────────────────────────────────────

const COL_STATUS: f32 = 40.0;
const COL_SIZE: f32 = 80.0;
const COL_PROGRESS: f32 = 150.0;
const COL_SPEED: f32 = 85.0;
const COL_REMAIN: f32 = 70.0;
const COL_ELAPSED: f32 = 70.0;
const COL_TYPE: f32 = 75.0;
const COL_SHA256: f32 = 130.0;
const ROW_H: f32 = 28.0;
const SIDEBAR_W: f32 = 140.0;

// ── Colors ───────────────────────────────────────────────────────────────────

const BLUE_PRIMARY: Color32 = Color32::from_rgb(30, 120, 230);
const BLUE_LIGHT: Color32 = Color32::from_rgb(220, 235, 252);
const BLUE_HEADER: Color32 = Color32::from_rgb(232, 238, 248);
const GRAY_BG: Color32 = Color32::from_rgb(245, 245, 245);
const GREEN: Color32 = Color32::from_rgb(76, 175, 80);
const RED_ERR: Color32 = Color32::from_rgb(229, 57, 53);

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Downloading,
    Completed,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum TaskStatus {
    Starting,
    Downloading,
    Paused,
    Completed,
    Error,
}

/// Serializable snapshot of a task for persistence.
#[derive(Serialize, Deserialize)]
struct TaskRecord {
    id: u64,
    url: String,
    filename: String,
    file_path: String,
    save_dir: String,
    file_size: u64,
    downloaded: u64,
    progress: f64,
    status: TaskStatus,
    error_msg: Option<String>,
    elapsed_secs: f64,
    #[serde(default = "default_task_count_val")]
    task_count: u64,
    #[serde(default)]
    cookies: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
}

fn default_task_count_val() -> u64 {
    15
}

#[allow(dead_code)]
struct DownloadTask {
    id: u64,
    url: String,
    filename: String,
    file_path: String,
    save_dir: String,
    file_size: u64,
    downloaded: u64,
    speed: u64,
    progress: f64,
    status: TaskStatus,
    error_msg: Option<String>,
    start_time: Instant,
    elapsed: Duration,
    receiver: Option<mpsc::Receiver<Result<DownloadFile, download_lib::DownloadError>>>,
    download: Option<DownloadFile>,
    logs: Vec<String>,
    task_count: u64,
    cookies: Option<String>,
    sha256: Option<String>,
    sha256_rx: Option<mpsc::Receiver<String>>,
}

enum Action {
    Select(u64),
    Pause(u64),
    Resume(u64),
    Delete(u64),
    Redownload(u64),
    OpenFile(String),
    OpenDir(String),
}

// ── App ──────────────────────────────────────────────────────────────────────

pub struct DurlApp {
    rt: tokio::runtime::Runtime,
    tasks: Vec<DownloadTask>,
    next_id: u64,
    filter: Filter,
    selected_task_id: Option<u64>,

    // New download dialog
    show_new_dialog: bool,
    new_url: String,
    new_save_path: String,
    new_filename: String,
    new_task_count: String,
    new_cookies: String,

    // Settings dialog
    show_settings: bool,

    // User config
    user_config: UserConfig,

    // i18n
    lang: LangStrings,

    // Log capture from download-lib
    log_buffer: LogBuffer,

    // Persistence
    dirty: bool,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn now_str() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

fn format_speed(bps: u64) -> String {
    if bps == 0 {
        return "—".into();
    }
    format!("{}/s", format_size(bps, BINARY))
}

fn format_dur(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

fn remaining_str(size: u64, down: u64, speed: u64) -> String {
    if speed == 0 || size <= down {
        return "—".into();
    }
    format_dur(Duration::from_secs((size - down) / speed))
}

fn file_ext(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("")
}

fn file_type_key(name: &str) -> &'static str {
    let ext = file_ext(name).to_ascii_lowercase();
    match ext.as_str() {
        "zip" | "rar" | "7z" | "tar" | "gz" | "xz" | "bz2" => "file_type.archive",
        "exe" | "msi" | "dmg" | "deb" | "rpm" | "apk" => "file_type.installer",
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => "file_type.video",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" => "file_type.audio",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico" => "file_type.image",
        "pdf" => "file_type.pdf",
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "file_type.document",
        "iso" | "img" => "file_type.disk_image",
        "txt" | "md" | "log" | "csv" | "json" | "xml" => "file_type.text",
        _ => "file_type.unknown",
    }
}

fn extract_filename(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}


// ── DurlApp implementation ──────────────────────────────────────────────────

impl DurlApp {
    pub fn new(cc: &eframe::CreationContext<'_>, log_buffer: LogBuffer, user_config: UserConfig) -> Self {
        Self::setup_fonts(&cc.egui_ctx);
        Self::setup_style(&cc.egui_ctx);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");

        let (tasks, next_id) = Self::load_tasks();
        let lang = LangStrings::load(&user_config.language);

        Self {
            rt,
            tasks,
            next_id,
            filter: Filter::All,
            selected_task_id: None,
            show_new_dialog: false,
            new_url: String::new(),
            new_save_path: user_config.default_save_path.clone(),
            new_filename: String::new(),
            new_task_count: user_config.default_task_count.to_string(),
            new_cookies: String::new(),
            show_settings: false,
            user_config,
            lang,
            log_buffer,
            dirty: false,
        }
    }

    fn setup_fonts(ctx: &egui::Context) {
        let candidates = [
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "C:\\Windows\\Fonts\\simsun.ttc",
        ];
        let mut fonts = egui::FontDefinitions::default();
        for path in &candidates {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert("chinese".into(), egui::FontData::from_owned(data).into());
                if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    f.insert(0, "chinese".into());
                }
                if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    f.push("chinese".into());
                }
                break;
            }
        }
        ctx.set_fonts(fonts);
    }

    fn setup_style(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::light();
        style.visuals.window_rounding = Rounding::same(6.0);
        style.visuals.widgets.noninteractive.rounding = Rounding::same(4.0);
        style.spacing.item_spacing = Vec2::new(6.0, 4.0);
        ctx.set_style(style);
    }

    // ── Persistence ─────────────────────────────────────────────────────────

    fn load_tasks() -> (Vec<DownloadTask>, u64) {
        let path = config_path();
        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return (Vec::new(), 1),
        };
        let records: Vec<TaskRecord> = match serde_json::from_str(&data) {
            Ok(r) => r,
            Err(_) => return (Vec::new(), 1),
        };

        let mut max_id = 0u64;
        let tasks: Vec<DownloadTask> = records
            .into_iter()
            .map(|r| {
                if r.id > max_id { max_id = r.id; }
                DownloadTask {
                    id: r.id, url: r.url, filename: r.filename,
                    file_path: r.file_path, save_dir: r.save_dir,
                    file_size: r.file_size, downloaded: r.downloaded,
                    speed: 0, progress: r.progress,
                    status: match r.status {
                        TaskStatus::Starting | TaskStatus::Downloading => TaskStatus::Paused,
                        other => other,
                    },
                    error_msg: r.error_msg,
                    start_time: Instant::now(),
                    elapsed: Duration::from_secs_f64(r.elapsed_secs),
                    receiver: None, download: None,
                    logs: vec![format!("[{}] Restored", now_str())],
                    task_count: r.task_count, cookies: r.cookies,
                    sha256: r.sha256,
                    sha256_rx: None,
                }
            })
            .collect();
        (tasks, max_id + 1)
    }

    fn save_tasks(&self) {
        let records: Vec<TaskRecord> = self.tasks.iter().map(|t| TaskRecord {
            id: t.id, url: t.url.clone(), filename: t.filename.clone(),
            file_path: t.file_path.clone(), save_dir: t.save_dir.clone(),
            file_size: t.file_size, downloaded: t.downloaded,
            progress: t.progress, status: t.status,
            error_msg: t.error_msg.clone(), elapsed_secs: t.elapsed.as_secs_f64(),
            task_count: t.task_count, cookies: t.cookies.clone(),
            sha256: t.sha256.clone(),
        }).collect();
        if let Ok(json) = serde_json::to_string_pretty(&records) {
            let _ = std::fs::write(config_path(), json);
        }
    }

    fn mark_dirty(&mut self) { self.dirty = true; }

    fn flush_if_dirty(&mut self) {
        if self.dirty { self.save_tasks(); self.dirty = false; }
    }

    // ── Task polling ────────────────────────────────────────────────────────

    fn drain_lib_logs(&mut self) {
        if let Ok(mut buf) = self.log_buffer.lock() {
            if buf.is_empty() { return; }
            let entries: Vec<_> = buf.drain(..).collect();
            drop(buf);

            let active_id = self.selected_task_id
                .filter(|sid| self.tasks.iter().any(|t| t.id == *sid && matches!(t.status, TaskStatus::Starting | TaskStatus::Downloading)))
                .or_else(|| self.tasks.iter().find(|t| matches!(t.status, TaskStatus::Starting | TaskStatus::Downloading)).map(|t| t.id));

            if let Some(id) = active_id {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    for entry in entries {
                        let level_tag = match entry.level {
                            log::Level::Error => "ERROR",
                            log::Level::Warn => "WARN",
                            log::Level::Info => "INFO",
                            log::Level::Debug => "DEBUG",
                            log::Level::Trace => "TRACE",
                        };
                        task.logs.push(format!("[{}] [{}] {}", entry.time, level_tag, entry.message));
                        if task.logs.len() > 2000 { task.logs.drain(0..500); }
                    }
                }
            }
        }
    }

    fn update_tasks(&mut self) {
        for task in &mut self.tasks {
            // Poll SHA256 computation result
            if let Some(rx) = &task.sha256_rx {
                if let Ok(hash) = rx.try_recv() {
                    task.sha256 = Some(hash.clone());
                    task.logs.push(format!("[{}] SHA256: {}", now_str(), hash));
                    task.sha256_rx = None;
                    self.dirty = true;
                }
            }

            if let Some(rx) = &task.receiver {
                if let Ok(result) = rx.try_recv() {
                    match result {
                        Ok(df) => {
                            let real = df.get_real_file_path();
                            task.filename = extract_filename(&real);
                            task.file_path = real;
                            task.file_size = df.size();
                            task.status = TaskStatus::Downloading;
                            task.logs.push(format!("[{}] Downloading: {}", now_str(), task.filename));
                            task.download = Some(df);
                        }
                        Err(e) => {
                            task.status = TaskStatus::Error;
                            task.error_msg = Some(e.to_string());
                            task.logs.push(format!("[{}] Failed: {}", now_str(), e));
                        }
                    }
                    task.receiver = None;
                    self.dirty = true;
                }
            }
            if let Some(df) = &task.download {
                let status = df.get_status();
                task.downloaded = status.get_down_size();
                if df.size() > 0 { task.file_size = df.size(); }
                task.speed = status.get_byte_sec();
                task.progress = status.get_percent_complete();
                // Only update elapsed when actively downloading
                if matches!(task.status, TaskStatus::Starting | TaskStatus::Downloading) {
                    task.elapsed = task.start_time.elapsed();
                }
                if status.is_finish() && task.status != TaskStatus::Completed && task.status != TaskStatus::Error {
                    if status.is_error() {
                        task.status = TaskStatus::Error;
                        task.error_msg = status.get_error().map(|e| e.to_string());
                        task.logs.push(format!("[{}] Error: {}", now_str(), task.error_msg.as_deref().unwrap_or("unknown")));
                    } else {
                        task.status = TaskStatus::Completed;
                        task.progress = 100.0;
                        task.downloaded = task.file_size;
                        task.speed = 0;
                        task.file_path = df.get_real_file_path();
                        task.logs.push(format!("[{}] Completed: {}", now_str(), task.file_path));
                        // Spawn SHA256 computation in background
                        if task.sha256.is_none() && task.sha256_rx.is_none() {
                            let path = task.file_path.clone();
                            let (tx, rx) = mpsc::channel();
                            std::thread::spawn(move || {
                                if let Ok(hash) = compute_sha256(&path) {
                                    let _ = tx.send(hash);
                                }
                            });
                            task.sha256_rx = Some(rx);
                            task.logs.push(format!("[{}] Computing SHA256...", now_str()));
                        }
                    }
                    self.dirty = true;
                }
            }
        }
    }

    // ── Download start ──────────────────────────────────────────────────────

    fn start_download(&mut self) {
        let url = self.new_url.trim().to_string();
        if url.is_empty() { return; }
        let save_path = PathBuf::from(self.new_save_path.trim());
        let task_count: u64 = self.new_task_count.trim().parse().unwrap_or(15);
        let custom_name = if self.new_filename.trim().is_empty() { None } else { Some(self.new_filename.trim().to_string()) };
        let cookies = if self.new_cookies.trim().is_empty() { None } else { Some(self.new_cookies.trim().to_string()) };

        let id = self.next_id;
        self.next_id += 1;

        let (tx, rx) = mpsc::channel();
        let url_clone = url.clone();
        let cookies_clone = cookies.clone();
        self.rt.spawn(async move {
            let result = DownloadFile::start_download(url_clone, save_path, task_count, 1024 * 1024, custom_name, cookies_clone).await;
            let _ = tx.send(result);
        });

        let display_name = if !self.new_filename.trim().is_empty() { self.new_filename.trim().to_string() } else { extract_filename(&url) };

        self.tasks.push(DownloadTask {
            id, url: url.clone(), filename: display_name, file_path: String::new(),
            save_dir: self.new_save_path.clone(), file_size: 0, downloaded: 0, speed: 0,
            progress: 0.0, status: TaskStatus::Starting, error_msg: None,
            start_time: Instant::now(), elapsed: Duration::ZERO,
            receiver: Some(rx), download: None,
            logs: vec![format!("[{}] Preparing: {}", now_str(), url)],
            task_count, cookies: cookies.clone(),
            sha256: None,
            sha256_rx: None,
        });

        self.selected_task_id = Some(id);
        self.mark_dirty();
        self.new_url.clear();
        self.new_filename.clear();
        self.new_cookies.clear();
        self.show_new_dialog = false;
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Select(id) => { self.selected_task_id = Some(id); }
            Action::Pause(id) => {
                if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                    if let Some(df) = &t.download { df.suspend(); }
                    t.status = TaskStatus::Paused;
                    t.speed = 0;
                    t.logs.push(format!("[{}] Paused", now_str()));
                }
                self.mark_dirty();
            }
            Action::Resume(id) => {
                if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                    if let Some(df) = &t.download {
                        df.restart();
                        t.status = TaskStatus::Downloading;
                        t.logs.push(format!("[{}] Resumed", now_str()));
                    } else {
                        let url = t.url.clone();
                        let save_path = PathBuf::from(&t.save_dir);
                        let task_count = t.task_count;
                        let custom_name = if t.filename.is_empty() { None } else { Some(t.filename.clone()) };
                        let cookies = t.cookies.clone();
                        let (tx, rx) = mpsc::channel();
                        self.rt.spawn(async move {
                            let result = DownloadFile::start_download(url, save_path, task_count, 1024 * 1024, custom_name, cookies).await;
                            let _ = tx.send(result);
                        });
                        t.receiver = Some(rx);
                        t.status = TaskStatus::Starting;
                        t.speed = 0; t.downloaded = 0; t.progress = 0.0;
                        t.start_time = Instant::now(); t.elapsed = Duration::ZERO;
                        t.error_msg = None;
                        t.logs.push(format!("[{}] Restarting", now_str()));
                    }
                }
                self.mark_dirty();
            }
            Action::Delete(id) => {
                if let Some(t) = self.tasks.iter().find(|t| t.id == id) {
                    if let Some(df) = &t.download { df.suspend(); }
                    if !t.file_path.is_empty() {
                        let p = PathBuf::from(&t.file_path);
                        let _ = std::fs::remove_file(&p);
                        let dd = p.with_extension("dd");
                        let _ = std::fs::remove_file(&dd);
                    }
                    if !t.save_dir.is_empty() && !t.filename.is_empty() {
                        let p = PathBuf::from(&t.save_dir).join(&t.filename);
                        let _ = std::fs::remove_file(&p);
                        let dd = p.with_extension("dd");
                        let _ = std::fs::remove_file(&dd);
                    }
                }
                self.tasks.retain(|t| t.id != id);
                if self.selected_task_id == Some(id) { self.selected_task_id = None; }
                self.mark_dirty();
            }
            Action::OpenFile(path) => { let _ = open::that(&path); }
            Action::OpenDir(path) => {
                #[cfg(windows)]
                { let _ = std::process::Command::new("explorer").args(["/select,", &path]).spawn(); }
                #[cfg(not(windows))]
                { if let Some(dir) = PathBuf::from(&path).parent() { let _ = open::that(dir); } }
            }
            Action::Redownload(id) => {
                if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                    // Delete existing file
                    if !t.file_path.is_empty() {
                        let _ = std::fs::remove_file(&t.file_path);
                    }
                    let url = t.url.clone();
                    let save_path = PathBuf::from(&t.save_dir);
                    let task_count = t.task_count;
                    let custom_name = if t.filename.is_empty() { None } else { Some(t.filename.clone()) };
                    let cookies = t.cookies.clone();
                    let (tx, rx) = mpsc::channel();
                    self.rt.spawn(async move {
                        let result = DownloadFile::start_download(url, save_path, task_count, 1024 * 1024, custom_name, cookies).await;
                        let _ = tx.send(result);
                    });
                    t.receiver = Some(rx);
                    t.download = None;
                    t.status = TaskStatus::Starting;
                    t.speed = 0; t.downloaded = 0; t.progress = 0.0;
                    t.start_time = Instant::now(); t.elapsed = Duration::ZERO;
                    t.error_msg = None;
                    t.sha256 = None; t.sha256_rx = None;
                    t.logs.push(format!("[{}] Re-downloading", now_str()));
                }
                self.mark_dirty();
            }
        }
    }

    fn task_visible(&self, t: &DownloadTask) -> bool {
        match self.filter {
            Filter::All => true,
            Filter::Downloading => matches!(t.status, TaskStatus::Starting | TaskStatus::Downloading | TaskStatus::Paused | TaskStatus::Error),
            Filter::Completed => t.status == TaskStatus::Completed,
        }
    }

    fn count_downloading(&self) -> usize {
        self.tasks.iter().filter(|t| matches!(t.status, TaskStatus::Starting | TaskStatus::Downloading | TaskStatus::Paused)).count()
    }

    fn count_completed(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count()
    }

    // ── UI render methods ───────────────────────────────────────────────────

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let btn_size = Vec2::new(90.0, 28.0);

            if ui.add_sized(btn_size, egui::Button::new(RichText::new(self.lang.get("toolbar.new_download")).size(13.0))).clicked() {
                self.show_new_dialog = true;
                self.new_save_path = self.user_config.default_save_path.clone();
            }
            ui.separator();

            let has_selected = self.selected_task_id.is_some();
            let sel_status = self.selected_task_id.and_then(|id| self.tasks.iter().find(|t| t.id == id).map(|t| t.status));
            let can_pause = sel_status == Some(TaskStatus::Downloading);
            let can_resume = sel_status == Some(TaskStatus::Paused);

            if ui.add_enabled(can_pause, egui::Button::new(RichText::new(self.lang.get("toolbar.pause")).size(13.0)).min_size(btn_size)).clicked() {
                if let Some(id) = self.selected_task_id { self.handle_action(Action::Pause(id)); }
            }
            if ui.add_enabled(can_resume, egui::Button::new(RichText::new(self.lang.get("toolbar.resume")).size(13.0)).min_size(btn_size)).clicked() {
                if let Some(id) = self.selected_task_id { self.handle_action(Action::Resume(id)); }
            }
            if ui.add_enabled(has_selected, egui::Button::new(RichText::new(self.lang.get("toolbar.delete")).size(13.0)).min_size(btn_size)).clicked() {
                if let Some(id) = self.selected_task_id { self.handle_action(Action::Delete(id)); }
            }
            ui.separator();

            if ui.add_sized(btn_size, egui::Button::new(RichText::new(self.lang.get("toolbar.settings")).size(13.0))).clicked() {
                self.show_settings = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let active = self.tasks.iter().filter(|t| t.status == TaskStatus::Downloading).count();
                let total_speed: u64 = self.tasks.iter().filter(|t| t.status == TaskStatus::Downloading).map(|t| t.speed).sum();
                if active > 0 {
                    let speed_str = format_speed(total_speed);
                    let text = self.lang.get_fmt("toolbar.active_speed", &[("active", &active.to_string()), ("speed", &speed_str)]);
                    ui.label(RichText::new(text).size(12.0).color(Color32::GRAY));
                }
            });
        });
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(self.lang.get("sidebar.my_downloads")).size(16.0).strong());
            ui.add_space(12.0);

            let items: [(Filter, String, usize); 3] = [
                (Filter::All, self.lang.get("sidebar.all_tasks").to_string(), self.tasks.len()),
                (Filter::Downloading, self.lang.get("sidebar.downloading").to_string(), self.count_downloading()),
                (Filter::Completed, self.lang.get("sidebar.completed").to_string(), self.count_completed()),
            ];

            for (f, label, count) in items {
                let selected = self.filter == f;
                let bg = if selected { BLUE_LIGHT } else { Color32::TRANSPARENT };
                let text_color = if selected { BLUE_PRIMARY } else { Color32::from_gray(60) };

                let resp = egui::Frame::none()
                    .fill(bg).rounding(Rounding::same(4.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&label).size(13.0).color(text_color));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if count > 0 {
                                    ui.label(RichText::new(count.to_string()).size(11.0).color(Color32::GRAY));
                                }
                            });
                        });
                    }).response;

                if resp.interact(egui::Sense::click()).clicked() { self.filter = f; }
                ui.add_space(2.0);
            }
        });
    }

    fn render_table(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_width();
        let fixed = COL_STATUS + COL_SIZE + COL_PROGRESS + COL_SPEED + COL_REMAIN + COL_ELAPSED + COL_TYPE + COL_SHA256 + 50.0;
        let col_name = (avail - fixed).max(150.0);

        egui::Frame::none().fill(BLUE_HEADER).inner_margin(egui::Margin::symmetric(4.0, 4.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized([COL_STATUS, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_status")).size(12.0).strong()));
                ui.add_sized([col_name, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_filename")).size(12.0).strong()));
                ui.add_sized([COL_SIZE, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_filesize")).size(12.0).strong()));
                ui.add_sized([COL_PROGRESS, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_progress")).size(12.0).strong()));
                ui.add_sized([COL_SPEED, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_speed")).size(12.0).strong()));
                ui.add_sized([COL_REMAIN, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_remaining")).size(12.0).strong()));
                ui.add_sized([COL_ELAPSED, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_elapsed")).size(12.0).strong()));
                ui.add_sized([COL_TYPE, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_filetype")).size(12.0).strong()));
                ui.add_sized([COL_SHA256, 20.0], egui::Label::new(RichText::new(self.lang.get("table.col_sha256")).size(12.0).strong()));
            });
        });

        let mut action: Option<Action> = None;

        // Snapshot keys before entering closures that borrow self
        let empty_all = self.lang.get("table.empty_all").to_string();
        let empty_downloading = self.lang.get("table.empty_downloading").to_string();
        let empty_completed = self.lang.get("table.empty_completed").to_string();
        let ctx_pause = self.lang.get("context_menu.pause").to_string();
        let ctx_resume = self.lang.get("context_menu.resume").to_string();
        let ctx_open_file = self.lang.get("context_menu.open_file").to_string();
        let ctx_open_dir = self.lang.get("context_menu.open_dir").to_string();
        let ctx_delete = self.lang.get("context_menu.delete").to_string();
        let ctx_copy_url = self.lang.get("context_menu.copy_url").to_string();
        let ctx_copy_sha256 = self.lang.get("context_menu.copy_sha256").to_string();
        let ctx_redownload = self.lang.get("context_menu.redownload").to_string();

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            let indices: Vec<usize> = (0..self.tasks.len()).filter(|&i| self.task_visible(&self.tasks[i])).collect();

            if indices.is_empty() {
                ui.add_space(60.0);
                ui.vertical_centered(|ui| {
                    let msg = match self.filter {
                        Filter::All => &empty_all,
                        Filter::Downloading => &empty_downloading,
                        Filter::Completed => &empty_completed,
                    };
                    ui.label(RichText::new(msg).size(14.0).color(Color32::from_gray(160)));
                });
                return;
            }

            for (row_idx, &task_idx) in indices.iter().enumerate() {
                let id = self.tasks[task_idx].id;
                let status = self.tasks[task_idx].status;
                let filename = self.tasks[task_idx].filename.clone();
                let file_size = self.tasks[task_idx].file_size;
                let downloaded = self.tasks[task_idx].downloaded;
                let progress = self.tasks[task_idx].progress;
                let speed = self.tasks[task_idx].speed;
                let elapsed = self.tasks[task_idx].elapsed;
                let file_path = self.tasks[task_idx].file_path.clone();
                let url = self.tasks[task_idx].url.clone();
                let sha256 = self.tasks[task_idx].sha256.clone();
                let is_selected = self.selected_task_id == Some(id);

                let bg = if is_selected { BLUE_LIGHT } else if row_idx % 2 == 0 { Color32::from_gray(252) } else { Color32::WHITE };

                let resp = egui::Frame::none().fill(bg).inner_margin(egui::Margin::symmetric(4.0, 0.0)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_min_height(ROW_H);
                        let (icon, color) = match status {
                            TaskStatus::Starting => ("⏳", Color32::from_gray(140)),
                            TaskStatus::Downloading => ("⬇", BLUE_PRIMARY),
                            TaskStatus::Paused => ("⏸", Color32::from_rgb(255, 152, 0)),
                            TaskStatus::Completed => ("✅", GREEN),
                            TaskStatus::Error => ("❌", RED_ERR),
                        };
                        ui.add_sized([COL_STATUS, ROW_H], egui::Label::new(RichText::new(icon).size(14.0).color(color)));
                        ui.add_sized([col_name, ROW_H], egui::Label::new(RichText::new(&filename).size(12.5)).truncate());
                        let size_str = if file_size > 0 { format_size(file_size, BINARY).to_string() } else { "—".into() };
                        ui.add_sized([COL_SIZE, ROW_H], egui::Label::new(RichText::new(size_str).size(12.0)));
                        ui.allocate_ui(Vec2::new(COL_PROGRESS, ROW_H), |ui| {
                            ui.centered_and_justified(|ui| {
                                let pct = (progress as f32 / 100.0).clamp(0.0, 1.0);
                                let bar = egui::ProgressBar::new(pct).text(format!("{:.1}%", progress))
                                    .fill(if status == TaskStatus::Completed { GREEN } else { BLUE_PRIMARY });
                                ui.add(bar);
                            });
                        });
                        ui.add_sized([COL_SPEED, ROW_H], egui::Label::new(RichText::new(format_speed(speed)).size(12.0)));
                        ui.add_sized([COL_REMAIN, ROW_H], egui::Label::new(RichText::new(remaining_str(file_size, downloaded, speed)).size(12.0)));
                        ui.add_sized([COL_ELAPSED, ROW_H], egui::Label::new(RichText::new(format_dur(elapsed)).size(12.0)));
                        ui.add_sized([COL_TYPE, ROW_H], egui::Label::new(RichText::new(self.lang.get(file_type_key(&filename))).size(12.0)));
                        ui.add_sized([COL_SHA256, ROW_H], egui::Label::new(RichText::new(self.tasks[task_idx].sha256.as_deref().unwrap_or("—")).size(12.0)));
                    });
                }).response;

                let resp = resp.interact(egui::Sense::click());
                if resp.clicked() { action = Some(Action::Select(id)); }
                if resp.double_clicked() && status == TaskStatus::Completed && !file_path.is_empty() {
                    action = Some(Action::OpenFile(file_path.clone()));
                }

                resp.context_menu(|ui| {
                    if ui.button(&ctx_copy_url).clicked() {
                        ui.output_mut(|o| o.copied_text = url.clone());
                        ui.close_menu();
                    }
                    if let Some(ref hash) = sha256 {
                        if ui.button(&ctx_copy_sha256).clicked() {
                            ui.output_mut(|o| o.copied_text = hash.clone());
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    match status {
                        TaskStatus::Downloading if ui.button(&ctx_pause).clicked() => {
                            action = Some(Action::Pause(id)); ui.close_menu();
                        }
                        TaskStatus::Paused if ui.button(&ctx_resume).clicked() => {
                            action = Some(Action::Resume(id)); ui.close_menu();
                        }
                        TaskStatus::Completed => {
                            if !file_path.is_empty() {
                                if ui.button(&ctx_open_file).clicked() { action = Some(Action::OpenFile(file_path.clone())); ui.close_menu(); }
                                if ui.button(&ctx_open_dir).clicked() { action = Some(Action::OpenDir(file_path.clone())); ui.close_menu(); }
                            }
                            if ui.button(&ctx_redownload).clicked() { action = Some(Action::Redownload(id)); ui.close_menu(); }
                        }
                        _ => {}
                    }
                    ui.separator();
                    if ui.button(&ctx_delete).clicked() { action = Some(Action::Delete(id)); ui.close_menu(); }
                });
            }
        });

        if let Some(a) = action { self.handle_action(a); }
    }

    fn render_log_panel(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.lang.get("log_panel.title")).size(12.0).strong());
            if let Some(id) = self.selected_task_id {
                if let Some(t) = self.tasks.iter().find(|t| t.id == id) {
                    ui.label(RichText::new(format!("— {}", t.filename)).size(11.0).color(Color32::GRAY));
                }
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink([false; 2]).stick_to_bottom(true).show(ui, |ui| {
            let logs: &[String] = if let Some(id) = self.selected_task_id {
                if let Some(t) = self.tasks.iter().find(|t| t.id == id) { &t.logs } else { &[] }
            } else { &[] };

            if logs.is_empty() {
                ui.label(RichText::new(self.lang.get("log_panel.empty")).size(12.0).color(Color32::from_gray(160)));
            } else {
                for line in logs {
                    ui.label(RichText::new(line).size(11.5).monospace());
                }
            }
        });
    }

    fn render_new_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_new_dialog { return; }

        let title = self.lang.get("dialog_new.title").to_string();
        let lbl_url = self.lang.get("dialog_new.url").to_string();
        let lbl_save = self.lang.get("dialog_new.save_dir").to_string();
        let lbl_fname = self.lang.get("dialog_new.filename").to_string();
        let lbl_fname_hint = self.lang.get("dialog_new.filename_hint").to_string();
        let lbl_conc = self.lang.get("dialog_new.concurrency").to_string();
        let lbl_cookie = self.lang.get("dialog_new.cookie").to_string();
        let lbl_cancel = self.lang.get("dialog_new.cancel").to_string();
        let lbl_start = self.lang.get("dialog_new.start").to_string();
        let lbl_browse = self.lang.get("dialog_new.browse").to_string();

        let mut open = true;
        egui::Window::new(&title).open(&mut open).resizable(true).collapsible(false)
            .default_width(520.0).min_width(400.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(&lbl_url);
                    ui.add(egui::TextEdit::singleline(&mut self.new_url)
                        .desired_width(ui.available_width())
                        .hint_text("https://example.com/file.zip"));
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(&lbl_save);
                    let browse_w = 50.0;
                    ui.add(egui::TextEdit::singleline(&mut self.new_save_path)
                        .desired_width(ui.available_width() - browse_w - 8.0));
                    if ui.button(&lbl_browse).clicked() {
                        if let Some(dir) = rfd::FileDialog::new().set_directory(&self.new_save_path).pick_folder() {
                            self.new_save_path = dir.display().to_string();
                        }
                    }
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(&lbl_fname);
                    ui.add(egui::TextEdit::singleline(&mut self.new_filename)
                        .desired_width(ui.available_width())
                        .hint_text(&lbl_fname_hint));
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(&lbl_conc);
                    ui.add(egui::TextEdit::singleline(&mut self.new_task_count).desired_width(80.0));
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(&lbl_cookie);
                    ui.add(egui::TextEdit::singleline(&mut self.new_cookies)
                        .desired_width(ui.available_width())
                        .hint_text(r#"optional, e.g. {"session":"abc"}"#));
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(!self.new_url.trim().is_empty(),
                            egui::Button::new(RichText::new(&lbl_start).color(Color32::WHITE)).fill(BLUE_PRIMARY)).clicked() {
                            self.start_download();
                        }
                        ui.add_space(8.0);
                        if ui.button(&lbl_cancel).clicked() { self.show_new_dialog = false; }
                    });
                });
                ui.add_space(4.0);
            });
        if !open { self.show_new_dialog = false; }
    }

    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_settings { return; }

        let title = self.lang.get("dialog_settings.title").to_string();
        let lbl_dir = self.lang.get("dialog_settings.default_dir").to_string();
        let lbl_threads = self.lang.get("dialog_settings.default_threads").to_string();
        let lbl_language = self.lang.get("dialog_settings.language").to_string();
        let lbl_log_level = self.lang.get("dialog_settings.log_level").to_string();
        let lbl_lang_dir = self.lang.get("dialog_settings.lang_dir").to_string();
        let lbl_lang_dir_hint = self.lang.get("dialog_settings.lang_dir_hint").to_string();
        let lbl_open_dir = self.lang.get("dialog_settings.open_dir").to_string();
        let lbl_ok = self.lang.get("dialog_settings.ok").to_string();
        let lbl_browse = self.lang.get("dialog_new.browse").to_string();
        let available_langs = available_languages();

        let mut open = true;
        egui::Window::new(&title).open(&mut open).resizable(false).collapsible(false)
            .default_width(460.0).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("settings_grid").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
                    ui.label(&lbl_dir);
                    ui.horizontal(|ui| {
                        ui.add_sized([300.0, 24.0], egui::TextEdit::singleline(&mut self.user_config.default_save_path));
                        if ui.button(&lbl_browse).clicked() {
                            if let Some(dir) = rfd::FileDialog::new().set_directory(&self.user_config.default_save_path).pick_folder() {
                                self.user_config.default_save_path = dir.display().to_string();
                            }
                        }
                    });
                    ui.end_row();

                    ui.label(&lbl_threads);
                    let mut tc_str = self.user_config.default_task_count.to_string();
                    if ui.add_sized([80.0, 24.0], egui::TextEdit::singleline(&mut tc_str)).changed() {
                        if let Ok(v) = tc_str.parse::<u64>() { self.user_config.default_task_count = v; }
                    }
                    ui.end_row();

                    ui.label(&lbl_language);
                    ui.horizontal(|ui| {
                        for (lang_id, display) in &available_langs {
                            if ui.selectable_label(self.user_config.language == *lang_id, display).clicked() {
                                self.user_config.language = lang_id.clone();
                                self.lang = LangStrings::load(lang_id);
                            }
                        }
                    });
                    ui.end_row();

                    ui.label(&lbl_log_level);
                    ui.horizontal(|ui| {
                        for &level in LOG_LEVELS {
                            if ui.selectable_label(self.user_config.log_level.eq_ignore_ascii_case(level), level).clicked() {
                                self.user_config.log_level = level.to_string();
                                crate::gui_logger::set_log_level(self.user_config.log_level_filter());
                            }
                        }
                    });
                    ui.end_row();

                    ui.label(&lbl_lang_dir);
                    ui.horizontal(|ui| {
                        let lang_path = crate::paths::lang_dir().display().to_string();
                        ui.label(RichText::new(&lang_path).size(11.0).color(Color32::GRAY));
                        if ui.button(&lbl_open_dir).clicked() {
                            let _ = open::that(&lang_path);
                        }
                    });
                    ui.end_row();
                });

                ui.add_space(2.0);
                ui.label(RichText::new(&lbl_lang_dir_hint).size(11.0).color(Color32::from_gray(130)).italics());

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(340.0);
                    if ui.add(egui::Button::new(RichText::new(&lbl_ok).color(Color32::WHITE)).fill(BLUE_PRIMARY)).clicked() {
                        self.new_task_count = self.user_config.default_task_count.to_string();
                        self.user_config.save();
                        self.show_settings = false;
                    }
                });
            });
        if !open { self.show_settings = false; }
    }
}

// ── eframe::App ──────────────────────────────────────────────────────────────

impl eframe::App for DurlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(200));
        self.update_tasks();
        self.drain_lib_logs();
        self.flush_if_dirty();

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_tasks();
        }

        egui::TopBottomPanel::top("toolbar").exact_height(42.0)
            .frame(egui::Frame::none().fill(Color32::WHITE).inner_margin(egui::Margin::symmetric(8.0, 6.0)))
            .show(ctx, |ui| { self.render_toolbar(ui); });

        egui::SidePanel::left("sidebar").resizable(true).default_width(SIDEBAR_W).min_width(130.0).max_width(250.0)
            .frame(egui::Frame::none().fill(GRAY_BG).inner_margin(egui::Margin::symmetric(8.0, 4.0)))
            .show(ctx, |ui| { self.render_sidebar(ui); });

        egui::TopBottomPanel::bottom("log_panel").resizable(true).default_height(120.0).min_height(60.0).max_height(300.0)
            .frame(egui::Frame::none().fill(Color32::WHITE).inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .stroke(egui::Stroke::new(1.0, Color32::from_gray(220))))
            .show(ctx, |ui| { self.render_log_panel(ui); });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::WHITE).inner_margin(egui::Margin::same(0.0)))
            .show(ctx, |ui| { self.render_table(ui); });

        self.render_new_dialog(ctx);
        self.render_settings_dialog(ctx);
    }
}

fn config_path() -> PathBuf {
    crate::paths::tasks_config_path()
}

fn compute_sha256(path: &str) -> Result<String, std::io::Error> {
    use sha2::{Sha256, Digest};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

