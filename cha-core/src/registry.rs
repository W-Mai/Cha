use crate::{
    Plugin,
    config::Config,
    plugins::{ComplexityAnalyzer, LengthAnalyzer},
};

/// Manages plugin registration and lifecycle.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    /// Build registry from config, applying thresholds.
    pub fn from_config(config: &Config) -> Self {
        let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();

        if config.is_enabled("length") {
            let mut p = LengthAnalyzer::default();
            if let Some(v) = config.get_usize("length", "max_function_lines") {
                p.max_function_lines = v;
            }
            if let Some(v) = config.get_usize("length", "max_class_methods") {
                p.max_class_methods = v;
            }
            if let Some(v) = config.get_usize("length", "max_class_lines") {
                p.max_class_lines = v;
            }
            if let Some(v) = config.get_usize("length", "max_file_lines") {
                p.max_file_lines = v;
            }
            plugins.push(Box::new(p));
        }

        if config.is_enabled("complexity") {
            let mut p = ComplexityAnalyzer::default();
            if let Some(v) = config.get_usize("complexity", "warn_threshold") {
                p.warn_threshold = v;
            }
            if let Some(v) = config.get_usize("complexity", "error_threshold") {
                p.error_threshold = v;
            }
            plugins.push(Box::new(p));
        }

        Self { plugins }
    }

    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }
}
