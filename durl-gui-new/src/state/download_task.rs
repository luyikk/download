//! Download task tracking — wraps download-lib with GUI-facing state.
//! Runtime-only data (receiver, download handle, sha256 channel) is stored
//! separately in `RuntimeData` so that `DownloadTask` remains `Clone + PartialEq`
//! for use in Dioxus signals.

use anyhow::Result;
use dioxus::prelude::info;
use download_lib::DownloadFile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

/// Represents the current status of a download task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum TaskStatus {
    Starting,
    Downloading,
    Paused,
    Completed,
    Error,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Starting => write!(f, "Starting"),
            TaskStatus::Downloading => write!(f, "Downloading"),
            TaskStatus::Paused => write!(f, "Paused"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Error => write!(f, "Error"),
        }
    }
}

/// Which filter is active in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Downloading,
    Completed,
}

/// Runtime-only data that can't be cloned / put in signals.
pub struct RuntimeData {
    pub receiver: Option<mpsc::Receiver<Result<DownloadFile, download_lib::DownloadError>>>,
    pub download: Option<DownloadFile>,
    pub sha256_rx: Option<mpsc::Receiver<String>>,
}

/// Global storage for runtime-only task data, keyed by task id.
static RUNTIME: std::sync::LazyLock<Mutex<HashMap<u64, RuntimeData>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn runtime_map() -> &'static Mutex<HashMap<u64, RuntimeData>> {
    &RUNTIME
}

/// A download task tracked in the GUI. Clone + PartialEq for Dioxus signals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: u64,
    pub url: String,
    pub filename: String,
    pub file_path: String,
    pub save_dir: String,
    pub file_size: u64,
    pub downloaded: u64,
    pub speed: u64,
    pub progress: f64,
    pub status: TaskStatus,
    pub error_msg: Option<String>,
    #[serde(skip, default = "zero_duration")]
    pub elapsed: Duration,
    #[serde(skip)]
    pub start_time_ms: u64,
    pub task_count: u64,
    pub cookies: Option<String>,
    pub sha256: Option<String>,
}

fn zero_duration() -> Duration {
    Duration::ZERO
}

// ── Runtime data helpers ──────────────────────────────────────

impl DownloadTask {
    pub fn with_runtime<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut RuntimeData) -> R,
    {
        let map = runtime_map();
        let mut guard = map.lock().unwrap();
        let rt = guard.entry(self.id).or_insert_with(|| RuntimeData {
            receiver: None,
            download: None,
            sha256_rx: None,
        });
        f(rt)
    }

    /// Static helper to access runtime data by task id.
    pub fn with_runtime_id<F, R>(id: u64, f: F) -> R
    where
        F: FnOnce(&mut RuntimeData) -> R,
    {
        let map = runtime_map();
        let mut guard = map.lock().unwrap();
        let rt = guard.entry(id).or_insert_with(|| RuntimeData {
            receiver: None,
            download: None,
            sha256_rx: None,
        });
        f(rt)
    }

    pub fn remove_runtime(id: u64) {
        let map = runtime_map();
        map.lock().unwrap().remove(&id);
    }
}

// ── Formatters ────────────────────────────────────────────────

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

pub fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return "—".into();
    }
    format!("{}/s", format_size(bytes_per_sec))
}

pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        return format!("{}s", secs);
    }
    if secs < 3600 {
        return format!("{}m {}s", secs / 60, secs % 60);
    }
    format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
}

pub fn extract_filename(url_or_path: &str) -> String {
    let name = url_or_path.rsplit('/').next().unwrap_or(url_or_path);
    let name = if let Some(idx) = name.find('?') {
        &name[..idx]
    } else {
        name
    };
    sanitize_filename(&urlencoding_decode(name))
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(hex as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            result.push(' ');
        } else {
            result.push(bytes[i] as char);
        }
        i += 1;
    }
    result
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\x00'..='\x1f' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim_end_matches(|c: char| c == '.' || c == ' ')
        .to_string()
}

// ── Persistence ───────────────────────────────────────────────

impl DownloadTask {
    /// Serialize all tasks to JSON.
    pub fn save_all(tasks: &[DownloadTask]) -> Result<()> {
        let path = crate::paths::tasks_config_path();
        info!("saved tasks to {}", path.display());
        let filter: Vec<&DownloadTask> = tasks.iter().collect();
        let toml = serde_json::to_string_pretty(&filter)?;
        std::fs::write(&path, toml)?;

        Ok(())
    }

    /// Load persisted tasks. Restored as Paused.
    pub fn load_all() -> Result<Vec<DownloadTask>> {
        let path = crate::paths::tasks_config_path();
        info!("loaded tasks from {}", path.display());
        if !std::fs::exists(&path)? {
            return Ok(vec![]);
        }
        let data = std::fs::read_to_string(&path)?;
        let mut tasks = serde_json::from_str::<Vec<DownloadTask>>(&data)?;
        tasks
            .iter_mut()
            .filter(|t| t.status == TaskStatus::Starting || t.status == TaskStatus::Downloading)
            .for_each(|t| t.status = TaskStatus::Paused);

        Ok(tasks)
    }
}

/// Compute SHA256 of a file.
pub fn compute_sha256(path: &str) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}
