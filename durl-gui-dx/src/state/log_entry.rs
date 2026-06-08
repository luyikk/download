//! Structured log entry — mirrors durl-gui's `LogEntry` + inline formatting.

use crate::gui_logger::now_str;
use log::Level;

/// A single log entry as displayed in the log panel.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub time: String,
    pub level: Level,
    pub message: String,
    pub target: String,
}

impl LogEntry {
    /// Create an application-level log entry (no `[LEVEL]` tag shown).
    pub fn app(message: impl Into<String>) -> Self {
        Self {
            time: now_str(),
            level: Level::Info,
            message: message.into(),
            target: "durl_gui_new".to_string(),
        }
    }

    /// Create an application error.
    pub fn app_error(message: impl Into<String>) -> Self {
        Self {
            time: now_str(),
            level: Level::Error,
            message: message.into(),
            target: "durl_gui_new".to_string(),
        }
    }
}

/// All valid log level names for the settings UI.
pub const LOG_LEVELS: &[&str] = &["Error", "Warn", "Info", "Debug", "Trace"];
