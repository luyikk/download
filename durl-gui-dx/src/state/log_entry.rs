//! Structured log entry — mirrors durl-gui's `LogEntry` + inline formatting.

use log::Level;

/// A single log entry as displayed in the log panel.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub time: String,
    pub level: Level,
    pub message: String,
    pub target: String,
}

/// All valid log level names for the settings UI.
pub const LOG_LEVELS: &[&str] = &["Error", "Warn", "Info", "Debug", "Trace"];
