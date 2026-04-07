use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level config from `.cha.toml`.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
}

/// Per-plugin config section.
#[derive(Debug, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub options: HashMap<String, toml::Value>,
}

fn default_true() -> bool {
    true
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            options: HashMap::new(),
        }
    }
}

impl Config {
    /// Load config from `.cha.toml` in the given directory, or return default.
    pub fn load(dir: &Path) -> Self {
        let path = dir.join(".cha.toml");
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Check if a plugin is enabled (default: true if not mentioned).
    pub fn is_enabled(&self, name: &str) -> bool {
        self.plugins.get(name).is_none_or(|c| c.enabled)
    }

    /// Get a usize option for a plugin.
    pub fn get_usize(&self, plugin: &str, key: &str) -> Option<usize> {
        self.plugins
            .get(plugin)?
            .options
            .get(key)?
            .as_integer()
            .map(|v| v as usize)
    }
}
