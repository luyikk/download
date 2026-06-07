use std::fmt;

/// Represents the current status of a download task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Filter::All => write!(f, "All"),
            Filter::Downloading => write!(f, "Downloading"),
            Filter::Completed => write!(f, "Completed"),
        }
    }
}

/// A mock download task for UI prototyping.
#[derive(Debug, Clone, PartialEq)]
pub struct MockTask {
    pub id: u64,
    pub filename: String,
    pub url: String,
    pub file_size: u64,  // total size in bytes
    pub downloaded: u64, // bytes downloaded so far
    pub progress: f64,   // 0.0 – 100.0
    pub speed: u64,      // bytes per second
    pub status: TaskStatus,
    pub elapsed_secs: u64,
    pub error_msg: Option<String>,
}

impl MockTask {
    /// Remaining bytes.
    pub fn remaining_bytes(&self) -> u64 {
        self.file_size.saturating_sub(self.downloaded)
    }

    /// Estimated time remaining in seconds, or None if speed is 0.
    pub fn eta_secs(&self) -> Option<u64> {
        if self.speed == 0 {
            return None;
        }
        Some(self.remaining_bytes() / self.speed)
    }
}

/// Format bytes to human-readable string.
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

/// Format speed to human-readable string.
pub fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return "—".into();
    }
    format!("{}/s", format_size(bytes_per_sec))
}

/// Format seconds to a human-readable duration string.
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        return format!("{}s", secs);
    }
    if secs < 3600 {
        return format!("{}m {}s", secs / 60, secs % 60);
    }
    format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
}

/// Generate 6 mock download tasks for UI prototyping.
pub fn mock_tasks() -> Vec<MockTask> {
    vec![
        MockTask {
            id: 1,
            filename: "ubuntu-24.04-desktop-amd64.iso".into(),
            url: "https://releases.ubuntu.com/24.04/ubuntu-24.04-desktop-amd64.iso".into(),
            file_size: 5_700_000_000,
            downloaded: 3_819_000_000,
            progress: 67.0,
            speed: 8_200_000,
            status: TaskStatus::Downloading,
            elapsed_secs: 467,
            error_msg: None,
        },
        MockTask {
            id: 2,
            filename: "music-collection.zip".into(),
            url: "https://example.com/music-collection.zip".into(),
            file_size: 380_000_000,
            downloaded: 45_600_000,
            progress: 12.0,
            speed: 1_500_000,
            status: TaskStatus::Downloading,
            elapsed_secs: 30,
            error_msg: None,
        },
        MockTask {
            id: 3,
            filename: "presentation.pptx".into(),
            url: "https://example.com/presentation.pptx".into(),
            file_size: 15_200_000,
            downloaded: 15_200_000,
            progress: 100.0,
            speed: 0,
            status: TaskStatus::Completed,
            elapsed_secs: 12,
            error_msg: None,
        },
        MockTask {
            id: 4,
            filename: "docker-desktop-installer.exe".into(),
            url: "https://desktop.docker.com/win/main/amd64/Docker Desktop Installer.exe".into(),
            file_size: 580_000_000,
            downloaded: 261_000_000,
            progress: 45.0,
            speed: 0,
            status: TaskStatus::Paused,
            elapsed_secs: 95,
            error_msg: None,
        },
        MockTask {
            id: 5,
            filename: "vacation-photos-2024.zip".into(),
            url: "https://example.com/photos/vacation-photos-2024.zip".into(),
            file_size: 1_200_000_000,
            downloaded: 1_068_000_000,
            progress: 89.0,
            speed: 3_100_000,
            status: TaskStatus::Downloading,
            elapsed_secs: 345,
            error_msg: None,
        },
        MockTask {
            id: 6,
            filename: "project-backup.tar.gz".into(),
            url: "https://example.com/backups/project-backup.tar.gz".into(),
            file_size: 950_000_000,
            downloaded: 120_000_000,
            progress: 12.6,
            speed: 0,
            status: TaskStatus::Error,
            elapsed_secs: 42,
            error_msg: Some("Connection timed out after 30 seconds".into()),
        },
    ]
}
