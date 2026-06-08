use crate::paths::app_config_dir;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Holds all translated strings loaded from a TOML language file.
/// Keys are dot-separated paths like "sidebar.downloads".
#[derive(Debug, Clone, PartialEq)]
pub struct LangStrings {
    map: HashMap<String, String>,
    pub lang_id: String,
}

impl LangStrings {
    /// Load a language by id from embedded TOML files.
    pub fn load(lang_id: &str) -> Self {
        let dir = lang_dir();
        let path = dir.join(format!("{}.toml", lang_id));

        // Auto-create default language files if missing
        ensure_default_lang_files(&dir);

        let map = match std::fs::read_to_string(&path) {
            Ok(data) => parse_toml_to_flat_map(&data),
            Err(_) => HashMap::new(),
        };
        Self {
            map,
            lang_id: lang_id.to_string(),
        }
    }

    /// Get a translated string by key. Returns the key itself if not found.
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.map.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Get a translated string with placeholder substitution.
    pub fn get_fmt(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let template = self.get(key);
        let mut result = template.to_string();
        for (name, value) in vars {
            result = result.replace(&format!("{{{}}}", name), value);
        }
        result
    }

    /// Return display name for the language.
    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        self.get("meta.display_name")
    }

    /// List available language files (returns list of lang ids like "zh-CN").
    pub fn available() -> Vec<(String, String)> {
        let dir = lang_dir();
        ensure_default_lang_files(&dir);

        let mut langs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let display = read_display_name(&path, stem);
                        langs.push((stem.to_string(), display));
                    }
                }
            }
        }
        if langs.is_empty() {
            langs.push(("zh-CN".into(), "中文".into()));
            langs.push(("en-US".into(), "English".into()));
        }
        langs.sort_by(|a, b| a.0.cmp(&b.0));
        langs
    }
}

/// Parse a TOML string into a flat HashMap with dot-separated keys.
fn parse_toml_to_flat_map(data: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(table) = data.parse::<toml::Table>() {
        flatten_table("", &table, &mut map);
    }
    map
}

fn flatten_table(prefix: &str, table: &toml::Table, map: &mut HashMap<String, String>) {
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };
        match value {
            toml::Value::String(s) => {
                map.insert(full_key, s.clone());
            }
            toml::Value::Table(t) => {
                flatten_table(&full_key, t, map);
            }
            _ => {}
        }
    }
}

/// Read the `meta.display_name` from a lang TOML file, or fall back to stem.
fn read_display_name(path: &Path, fallback: &str) -> String {
    if let Ok(data) = std::fs::read_to_string(path) {
        if let Ok(table) = data.parse::<toml::Table>() {
            if let Some(toml::Value::Table(meta)) = table.get("meta") {
                if let Some(toml::Value::String(name)) = meta.get("display_name") {
                    return name.clone();
                }
            }
        }
    }
    fallback.to_string()
}

/// Ensure that default language files exist and are up-to-date.
/// If a file is missing or its content differs from the embedded default, regenerate it.
fn ensure_default_lang_files(dir: &Path) {
    let files: &[(&str, &str)] = &[
        ("zh-CN.toml", DEFAULT_ZH_CN),
        ("en-US.toml", DEFAULT_EN_US),
        ("ru-RU.toml", DEFAULT_RU_RU),
    ];
    for (filename, content) in files {
        let path = dir.join(filename);
        if should_regenerate(&path, content) {
            let _ = std::fs::write(&path, content);
        }
    }
}

/// Returns true if the file doesn't exist or its content differs from expected.
fn should_regenerate(path: &PathBuf, expected: &str) -> bool {
    match std::fs::read_to_string(path) {
        Ok(content) => content != expected,
        Err(_) => true,
    }
}

/// Directory containing language TOML files.
pub fn lang_dir() -> PathBuf {
    let dir = app_config_dir().join("lang");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// ── Embedded default language files ────────────────────────

const DEFAULT_EN_US: &str = include_str!("../../lang/en-US.toml");
const DEFAULT_ZH_CN: &str = include_str!("../../lang/zh-CN.toml");
const DEFAULT_RU_RU: &str = include_str!("../../lang/ru-RU.toml");
