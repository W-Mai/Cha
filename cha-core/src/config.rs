use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Strictness level — controls threshold scaling.
/// `relaxed` = 2.0x (thresholds doubled, more lenient),
/// `default` = 1.0x, `strict` = 0.5x (thresholds halved).
/// Can also be a custom float like `0.7`.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Strictness {
    Named(StrictnessLevel),
    Custom(f64),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrictnessLevel {
    Relaxed,
    Default,
    Strict,
}

impl Default for Strictness {
    fn default() -> Self {
        Strictness::Named(StrictnessLevel::Default)
    }
}

impl Strictness {
    pub fn factor(&self) -> f64 {
        match self {
            Strictness::Named(StrictnessLevel::Relaxed) => 2.0,
            Strictness::Named(StrictnessLevel::Default) => 1.0,
            Strictness::Named(StrictnessLevel::Strict) => 0.5,
            Strictness::Custom(v) => *v,
        }
    }

    /// Parse from CLI string: "relaxed", "default", "strict", or a float.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "relaxed" => Some(Strictness::Named(StrictnessLevel::Relaxed)),
            "default" => Some(Strictness::Named(StrictnessLevel::Default)),
            "strict" => Some(Strictness::Named(StrictnessLevel::Strict)),
            _ => s.parse::<f64>().ok().map(Strictness::Custom),
        }
    }
}

/// Per-language config overlay.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct LanguageConfig {
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
    /// Smell names disabled for this language (appended to global disabled_smells).
    #[serde(default)]
    pub disabled_smells: Vec<String>,
}

/// Top-level config from `.cha.toml`.
#[derive(Debug, Default, Clone, Deserialize)]
// cha:ignore large_class
pub struct Config {
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
    /// Glob patterns for paths to exclude from analysis.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Custom remediation time weights (minutes per severity).
    #[serde(default)]
    pub debt_weights: DebtWeights,
    /// Threshold scaling factor.
    #[serde(default)]
    pub strictness: Strictness,
    /// Per-language plugin overrides.
    #[serde(default)]
    pub languages: HashMap<String, LanguageConfig>,
    /// Smell names disabled globally (any language/plugin).
    /// Use when a plugin produces multiple smells and you want to suppress only some.
    #[serde(default)]
    pub disabled_smells: Vec<String>,
    /// Manual layer/module configuration for `cha layers`.
    #[serde(default)]
    pub layers: LayersConfig,
}

/// Manual module and tier definitions for architecture analysis.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct LayersConfig {
    /// Module name → file glob patterns.
    #[serde(default)]
    pub modules: HashMap<String, Vec<String>>,
    /// Tier name → module names (ordered bottom to top in config).
    #[serde(default)]
    pub tiers: Vec<TierConfig>,
}

/// A single tier definition.
#[derive(Debug, Clone, Deserialize)]
pub struct TierConfig {
    pub name: String,
    pub modules: Vec<String>,
}

/// Custom debt estimation weights (minutes per severity level).
#[derive(Debug, Clone, Deserialize)]
pub struct DebtWeights {
    #[serde(default = "default_hint_debt")]
    pub hint: u32,
    #[serde(default = "default_warning_debt")]
    pub warning: u32,
    #[serde(default = "default_error_debt")]
    pub error: u32,
}

fn default_hint_debt() -> u32 {
    5
}
fn default_warning_debt() -> u32 {
    15
}
fn default_error_debt() -> u32 {
    30
}

impl Default for DebtWeights {
    fn default() -> Self {
        Self {
            hint: 5,
            warning: 15,
            error: 30,
        }
    }
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

    /// Load config from a specific file path.
    pub fn load_file(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
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
        self.exclude.extend(other.exclude);
        self.debt_weights = other.debt_weights;
        // Only override strictness if the other config explicitly set it (non-default).
        // Since we can't distinguish "not set" from "set to default" with serde,
        // we always take the other's value during merge.
        self.strictness = other.strictness;
        self.disabled_smells.extend(other.disabled_smells);
        for (lang, other_lc) in other.languages {
            let entry = self.languages.entry(lang).or_default();
            for (name, other_pc) in other_lc.plugins {
                let pe = entry.plugins.entry(name).or_default();
                pe.enabled = other_pc.enabled;
                for (k, v) in other_pc.options {
                    pe.options.insert(k, v);
                }
            }
            entry.disabled_smells.extend(other_lc.disabled_smells);
        }
    }

    /// Produce a resolved config for a specific language.
    /// Applies builtin language profile first, then user overrides.
    pub fn resolve_for_language(&self, language: &str) -> Config {
        let lang_key = language.to_lowercase();
        let mut resolved = self.clone();
        self.apply_builtin_profile(&lang_key, &mut resolved);
        self.apply_user_language_overrides(&lang_key, &mut resolved);
        resolved
    }

    fn apply_builtin_profile(&self, lang_key: &str, resolved: &mut Config) {
        let Some(builtin) = builtin_language_profile(lang_key) else {
            return;
        };
        for (name, enabled, options) in builtin {
            let user_override = self
                .languages
                .get(lang_key)
                .is_some_and(|lc| lc.plugins.contains_key(name));
            if user_override {
                continue;
            }
            let entry = resolved.plugins.entry(name.to_string()).or_default();
            entry.enabled = enabled;
            for &(k, v) in options {
                entry
                    .options
                    .entry(k.to_string())
                    .or_insert(toml::Value::Integer(v));
            }
        }
    }

    fn apply_user_language_overrides(&self, lang_key: &str, resolved: &mut Config) {
        let Some(lc) = self.languages.get(lang_key) else {
            return;
        };
        for (name, lpc) in &lc.plugins {
            let entry = resolved.plugins.entry(name.clone()).or_default();
            entry.enabled = lpc.enabled;
            for (k, v) in &lpc.options {
                entry.options.insert(k.clone(), v.clone());
            }
        }
    }

    /// Check if a plugin is enabled (default: true if not mentioned).
    pub fn is_enabled(&self, name: &str) -> bool {
        self.plugins.get(name).is_none_or(|c| c.enabled)
    }

    /// All disabled smells effective for a given file language:
    /// global + language-level + builtin language profile's smell disables.
    /// Duplicates are kept; callers use Vec::contains() so it's fine.
    pub fn disabled_smells_for_language(&self, language: &str) -> Vec<String> {
        let lang_key = language.to_lowercase();
        let mut out = self.disabled_smells.clone();
        if let Some(lc) = self.languages.get(&lang_key) {
            out.extend(lc.disabled_smells.clone());
        }
        if let Some(builtin) = builtin_language_smell_disables(&lang_key) {
            out.extend(builtin.iter().map(|s| s.to_string()));
        }
        out
    }

    /// Check if a smell is disabled for the given file language.
    pub fn is_smell_disabled(&self, smell_name: &str, language: &str) -> bool {
        self.disabled_smells_for_language(language)
            .iter()
            .any(|s| s == smell_name)
    }

    /// Get a usize option for a plugin, scaled by strictness factor.
    pub fn get_usize(&self, plugin: &str, key: &str) -> Option<usize> {
        self.plugins
            .get(plugin)?
            .options
            .get(key)?
            .as_integer()
            .map(|v| {
                let scaled = (v as f64 * self.strictness.factor()).round() as usize;
                scaled.max(1)
            })
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

    /// Override strictness (e.g. from CLI --strictness flag).
    pub fn set_strictness(&mut self, s: Strictness) {
        self.strictness = s;
    }

    /// Apply calibration defaults — only sets values not already configured by user.
    pub fn set_calibration_defaults(&mut self, lines: usize, complexity: usize, cognitive: usize) {
        self.plugins
            .entry("length".into())
            .or_default()
            .options
            .entry("max_function_lines".into())
            .or_insert(toml::Value::Integer(lines as i64));
        self.plugins
            .entry("complexity".into())
            .or_default()
            .options
            .entry("max_complexity".into())
            .or_insert(toml::Value::Integer(complexity as i64));
        self.plugins
            .entry("cognitive_complexity".into())
            .or_default()
            .options
            .entry("max_cognitive_complexity".into())
            .or_insert(toml::Value::Integer(cognitive as i64));
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

/// A builtin plugin profile entry: (name, enabled, option overrides).
pub type PluginProfile = (&'static str, bool, &'static [(&'static str, i64)]);

/// Builtin language profiles: default plugin settings for specific languages.
/// Returns (plugin_name, enabled, options) tuples. Users can override via `[languages.xx.plugins.yy]`.
/// Only real plugin names allowed — smell-level disables go in `builtin_language_smell_disables`.
pub fn builtin_language_profile(language: &str) -> Option<Vec<PluginProfile>> {
    match language {
        "c" | "cpp" => Some(vec![
            ("naming", false, &[] as &[(&str, i64)]),
            // lazy_class / data_class stay disabled for C: they run as
            // per-file plugins and don't see the `c_oop_enrich` enrichment
            // (which only reaches post-analysis detectors via ProjectIndex).
            // When these plugins get migrated to post-analysis, reopen them.
            ("lazy_class", false, &[]),
            ("data_class", false, &[]),
            (
                "length",
                true,
                &[
                    ("max_function_lines", 100),
                    ("max_file_lines", 2000),
                    ("max_class_lines", 400),
                ],
            ),
            (
                "complexity",
                true,
                &[("warn_threshold", 15), ("error_threshold", 30)],
            ),
            ("cognitive_complexity", true, &[("threshold", 25)]),
            ("coupling", true, &[("max_imports", 25)]),
            ("long_parameter_list", true, &[("max_params", 7)]),
        ]),
        _ => None,
    }
}

/// Builtin smell-level disables for specific languages.
/// Use when a plugin emits multiple smells but only some should be suppressed.
pub fn builtin_language_smell_disables(language: &str) -> Option<&'static [&'static str]> {
    match language {
        "c" | "cpp" => Some(&[
            // design_pattern smells — C/C++ idioms don't match these pattern shapes
            "builder_pattern",
            "null_object_pattern",
            "strategy_pattern",
            // data_clumps — C functions with many params grouped is common and intentional
            "data_clumps",
        ]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_smells_builtin_for_c() {
        let cfg = Config::default();
        assert!(cfg.is_smell_disabled("builder_pattern", "c"));
        assert!(cfg.is_smell_disabled("data_clumps", "cpp"));
        assert!(!cfg.is_smell_disabled("builder_pattern", "rust"));
    }

    #[test]
    fn disabled_smells_global_applies_to_any_language() {
        let mut cfg = Config::default();
        cfg.disabled_smells.push("long_method".into());
        assert!(cfg.is_smell_disabled("long_method", "rust"));
        assert!(cfg.is_smell_disabled("long_method", "typescript"));
    }

    #[test]
    fn disabled_smells_language_level_overlay() {
        let mut cfg = Config::default();
        let mut lc = LanguageConfig::default();
        lc.disabled_smells.push("observer_pattern".into());
        cfg.languages.insert("go".into(), lc);
        assert!(cfg.is_smell_disabled("observer_pattern", "go"));
        assert!(!cfg.is_smell_disabled("observer_pattern", "rust"));
    }

    #[test]
    fn disabled_smells_combines_global_language_and_builtin() {
        let mut cfg = Config::default();
        cfg.disabled_smells.push("global_smell".into());
        let mut lc = LanguageConfig::default();
        lc.disabled_smells.push("lang_smell".into());
        cfg.languages.insert("c".into(), lc);
        let all = cfg.disabled_smells_for_language("c");
        assert!(all.iter().any(|s| s == "global_smell"));
        assert!(all.iter().any(|s| s == "lang_smell"));
        assert!(all.iter().any(|s| s == "builder_pattern")); // from builtin
    }

    #[test]
    fn merge_appends_disabled_smells() {
        let mut a = Config::default();
        a.disabled_smells.push("a_smell".into());
        let mut b = Config::default();
        b.disabled_smells.push("b_smell".into());
        a.merge(b);
        assert!(a.disabled_smells.contains(&"a_smell".into()));
        assert!(a.disabled_smells.contains(&"b_smell".into()));
    }
}
