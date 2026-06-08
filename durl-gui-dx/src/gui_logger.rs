//! Channel-based `log`-crate integration — mirrors durl-gui's GuiLogger.
//!
//! A background thread drains log records from a bounded channel into a shared
//! buffer. The main UI loop calls `drain_buffer()` each tick to collect them.

use crate::state::log_entry::LogEntry;
use log::{Level, Log, Metadata, Record};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// Thread-safe shared log buffer consumed by the UI.
pub type LogBuffer = Arc<Mutex<Vec<LogEntry>>>;

/// Custom logger that sends entries through an mpsc channel (non-blocking).
struct GuiLogger {
    sender: mpsc::SyncSender<LogEntry>,
}

/// Global atomic for the runtime log level.
static LOG_LEVEL: AtomicUsize = AtomicUsize::new(log::LevelFilter::Info as usize);

impl Log for GuiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let current = LOG_LEVEL.load(Ordering::Relaxed);
        (metadata.level() as usize) <= current
    }

    fn log(&self, record: &Record) {
        let entry = LogEntry {
            time: now_str(),
            level: record.level(),
            message: format!("{}", record.args()),
            target: record.module_path().unwrap_or(record.target()).to_owned(),
        };
        // Non-blocking send; drops the entry if the channel is full
        let _ = self.sender.try_send(entry);
    }

    fn flush(&self) {}
}

/// Initialize the GUI logger. Must be called once before any logging.
/// Returns the shared buffer; call `drain_buffer()` each tick to collect entries.
pub fn init_gui_logger(level: log::LevelFilter) -> LogBuffer {
    let buffer: LogBuffer = Arc::new(Mutex::new(Vec::with_capacity(512)));
    let (tx, rx) = mpsc::sync_channel::<LogEntry>(4096);

    // Background thread drains the channel into the shared buffer
    let buf_clone = buffer.clone();
    std::thread::Builder::new()
        .name("gui-logger".into())
        .spawn(move || {
            while let Ok(entry) = rx.recv() {
                // Filter: download_lib and durl_gui (includes browser_server) use
                // the configured level; everything else is capped at Error.
                let is_local = entry.target.starts_with("download_lib")
                    || entry.target.starts_with("durl_gui");

                if entry.message == "start" {
                    println!("[{}] {} - {}", entry.time, entry.level, entry.message);
                }

                let current = LOG_LEVEL.load(Ordering::Relaxed);
                let allowed = if is_local {
                    (entry.level as usize) <= current
                } else {
                    entry.level <= Level::Error
                };
                if !allowed {
                    continue;
                }
                if let Ok(mut buf) = buf_clone.lock() {
                    // Keep buffer bounded
                    if buf.len() > 5000 {
                        buf.drain(0..1000);
                    }
                    buf.push(entry);
                }
            }
        })
        .expect("failed to spawn gui-logger thread");

    let logger = GuiLogger { sender: tx };
    LOG_LEVEL.store(level as usize, Ordering::Relaxed);
    log::set_boxed_logger(Box::new(logger)).ok();
    // Set max to Trace so our atomic filter controls the actual level
    log::set_max_level(log::LevelFilter::Trace);
    buffer
}

/// Drain lib-generated log entries from the shared buffer into the UI.
pub fn drain_buffer(buffer: &LogBuffer) -> Vec<LogEntry> {
    if let Ok(mut buf) = buffer.lock() {
        if buf.is_empty() {
            return Vec::new();
        }
        let entries: Vec<_> = buf.drain(..).collect();
        entries
    } else {
        Vec::new()
    }
}

/// Change the runtime log level. Can be called from anywhere.
pub fn set_log_level(level: log::LevelFilter) {
    LOG_LEVEL.store(level as usize, Ordering::Relaxed);
}

pub fn now_str() -> String {
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
