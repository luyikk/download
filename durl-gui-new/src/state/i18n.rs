use std::collections::HashMap;

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
        let data = match lang_id {
            "zh-CN" => DEFAULT_ZH_CN,
            _ => DEFAULT_EN_US, // default to English
        };
        let map = parse_toml_to_flat_map(data);
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
    pub fn display_name(&self) -> &str {
        self.get("meta.display_name")
    }

    /// Available languages.
    pub fn available() -> Vec<(&'static str, &'static str)> {
        vec![("en-US", "English"), ("zh-CN", "中文")]
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

// ── Embedded default language files ────────────────────────

const DEFAULT_EN_US: &str = include_str!("../../lang/en-US.toml");
const DEFAULT_ZH_CN: &str = include_str!("../../lang/zh-CN.toml");
