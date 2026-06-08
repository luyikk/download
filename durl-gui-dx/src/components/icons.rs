/// File type metadata: display emoji + accent color class.
pub struct FileType {
    pub emoji: &'static str,
    pub color_class: &'static str,
}

/// Map a filename to its file type info based on extension.
pub fn file_type_for(filename: &str) -> FileType {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    match ext.as_str() {
        // Archives
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "tgz" => FileType {
            emoji: "📦",
            color_class: "bg-amber-500/20 text-amber-400",
        },
        // Audio
        "mp3" | "flac" | "wav" | "aac" | "ogg" | "wma" | "m4a" => FileType {
            emoji: "🎵",
            color_class: "bg-pink-500/20 text-pink-400",
        },
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => FileType {
            emoji: "🎬",
            color_class: "bg-red-500/20 text-red-400",
        },
        // Disk images
        "iso" | "img" | "dmg" | "vmdk" => FileType {
            emoji: "💿",
            color_class: "bg-blue-500/20 text-blue-400",
        },
        // Executables / installers
        "exe" | "msi" | "deb" | "rpm" | "apk" | "appimage" => FileType {
            emoji: "⚙️",
            color_class: "bg-slate-500/20 text-slate-400",
        },
        // Documents
        "pdf" => FileType {
            emoji: "📕",
            color_class: "bg-red-500/20 text-red-400",
        },
        "doc" | "docx" => FileType {
            emoji: "📝",
            color_class: "bg-blue-500/20 text-blue-400",
        },
        "xls" | "xlsx" | "csv" => FileType {
            emoji: "📊",
            color_class: "bg-emerald-500/20 text-emerald-400",
        },
        "ppt" | "pptx" => FileType {
            emoji: "📽️",
            color_class: "bg-orange-500/20 text-orange-400",
        },
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" => FileType {
            emoji: "🖼️",
            color_class: "bg-purple-500/20 text-purple-400",
        },
        // Code / text
        "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" | "toml" | "json"
        | "yaml" | "yml" | "xml" | "html" | "css" | "sh" | "bat" => FileType {
            emoji: "💻",
            color_class: "bg-cyan-500/20 text-cyan-400",
        },
        // Torrent
        "torrent" => FileType {
            emoji: "🔗",
            color_class: "bg-lime-500/20 text-lime-400",
        },
        // Default
        _ => FileType {
            emoji: "📄",
            color_class: "bg-slate-500/20 text-slate-400",
        },
    }
}
