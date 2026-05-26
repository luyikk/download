use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Holds all translated strings loaded from a TOML language file.
/// Keys are dot-separated paths like "sidebar.my_downloads".
pub struct LangStrings {
    map: HashMap<String, String>,
    #[allow(dead_code)]
    pub lang_id: String,
}

impl LangStrings {
    /// Load a language file by id (e.g. "zh-CN", "en-US").
    /// If the file doesn't exist, auto-create default language files.
    pub fn load(lang_id: &str) -> Self {
        let dir = crate::paths::lang_dir();
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
    /// Placeholders are `{name}` in the TOML value.
    pub fn get_fmt(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let template = self.get(key);
        let mut result = template.to_string();
        for (name, value) in vars {
            result = result.replace(&format!("{{{}}}", name), value);
        }
        result
    }
}

/// List available language files (returns list of lang ids like "zh-CN").
pub fn available_languages() -> Vec<(String, String)> {
    let dir = crate::paths::lang_dir();
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
    let zh_path = dir.join("zh-CN.toml");
    if should_regenerate(&zh_path, DEFAULT_ZH_CN) {
        let _ = std::fs::write(&zh_path, DEFAULT_ZH_CN);
    }
    let en_path = dir.join("en-US.toml");
    if should_regenerate(&en_path, DEFAULT_EN_US) {
        let _ = std::fs::write(&en_path, DEFAULT_EN_US);
    }
}

/// Returns true if the file doesn't exist or its content differs from expected.
fn should_regenerate(path: &PathBuf, expected: &str) -> bool {
    match std::fs::read_to_string(path) {
        Ok(content) => content != expected,
        Err(_) => true,
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

// ── Embedded default language files (compiled from lang/*.toml) ──────────────

const DEFAULT_ZH_CN: &str = include_str!("../lang/zh-CN.toml");

const DEFAULT_EN_US: &str = include_str!("../lang/en-US.toml");
