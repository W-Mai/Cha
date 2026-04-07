use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level config from `.cha.toml`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
}

/// Per-plugin config section.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(flatten)]
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

    /// Load merged config for a specific file by walking up from its directory to root.
    /// Child configs override parent configs (child-wins merge).
    pub fn load_for_file(file_path: &Path, project_root: &Path) -> Self {
        let abs_file = std::fs::canonicalize(file_path).unwrap_or(file_path.to_path_buf());
        let abs_root = std::fs::canonicalize(project_root).unwrap_or(project_root.to_path_buf());
        let dir = abs_file.parent().unwrap_or(&abs_root);

        // Merge: root is base, closest wins
        let mut configs = collect_configs_upward(dir, &abs_root);
        configs.reverse();
        let mut merged = Config::default();
        for cfg in configs {
            merged.merge(cfg);
        }
        merged
    }

    /// Merge another config into self. `other` values take precedence.
    pub fn merge(&mut self, other: Config) {
        for (name, other_pc) in other.plugins {
            let entry = self.plugins.entry(name).or_default();
            entry.enabled = other_pc.enabled;
            for (k, v) in other_pc.options {
                entry.options.insert(k, v);
            }
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

    /// Get a string option for a plugin.
    pub fn get_str(&self, plugin: &str, key: &str) -> Option<String> {
        self.plugins
            .get(plugin)?
            .options
            .get(key)?
            .as_str()
            .map(|s| s.to_string())
    }
}

/// Walk from `start_dir` up to `root`, collecting `.cha.toml` configs (closest first).
fn collect_configs_upward(start_dir: &Path, root: &Path) -> Vec<Config> {
    let mut configs = Vec::new();
    let mut current = start_dir.to_path_buf();
    loop {
        let cfg_path = current.join(".cha.toml");
        if cfg_path.is_file()
            && let Ok(content) = std::fs::read_to_string(&cfg_path)
            && let Ok(cfg) = toml::from_str::<Config>(&content)
        {
            configs.push(cfg);
        }
        if current == root {
            break;
        }
        match current.parent() {
            Some(p) if p.starts_with(root) || p == root => current = p.to_path_buf(),
            _ => break,
        }
    }
    configs
}
