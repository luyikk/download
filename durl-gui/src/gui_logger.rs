use log::{Level, Log, Metadata, Record};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// A log record captured from the `log` crate.
#[derive(Clone)]
pub struct LogEntry {
    pub time: String,
    pub level: Level,
    pub target: String,
    pub message: String,
}

/// Thread-safe shared log buffer.
pub type LogBuffer = Arc<Mutex<Vec<LogEntry>>>;

/// Custom logger that sends log messages through a channel to avoid blocking.
struct GuiLogger {
    sender: mpsc::SyncSender<LogEntry>,
}

/// Global atomic for the runtime log level (stored as usize for AtomicUsize).
static LOG_LEVEL: AtomicUsize = AtomicUsize::new(log::LevelFilter::Info as usize);

impl Log for GuiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let current = LOG_LEVEL.load(Ordering::Relaxed);
        (metadata.level() as usize) <= current
    }

    fn log(&self, record: &Record) {
        let entry = LogEntry {
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            level: record.level(),
            target: record.module_path().unwrap_or(record.target()).to_owned(),
            message: format!("{}", record.args()),
        };
        // Non-blocking send; drops the entry if the channel is full
        let _ = self.sender.try_send(entry);
    }

    fn flush(&self) {}
}

/// Initialize the custom GUI logger. Must be called once before any logging.
/// Returns the shared buffer that the GUI reads from.
pub fn init_gui_logger(level: log::LevelFilter) -> LogBuffer {
    let buffer: LogBuffer = Arc::new(Mutex::new(Vec::with_capacity(512)));
    let (tx, rx) = mpsc::sync_channel::<LogEntry>(4096);

    // Background thread drains the channel into the shared buffer
    let buf_clone = buffer.clone();
    std::thread::Builder::new()
        .name("gui-logger".into())
        .spawn(move || {
            while let Ok(entry) = rx.recv() {
                // Filter: download_lib uses configured level, others only Error
                let is_download_lib = entry.target.starts_with("download_lib");
                let current = LOG_LEVEL.load(Ordering::Relaxed);
                let allowed = if is_download_lib {
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

/// Change the runtime log level. Can be called from anywhere.
pub fn set_log_level(level: log::LevelFilter) {
    LOG_LEVEL.store(level as usize, Ordering::Relaxed);
}
