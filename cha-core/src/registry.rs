use std::path::Path;

use crate::{
    Plugin,
    config::Config,
    plugins::{
        ApiSurfaceAnalyzer, ComplexityAnalyzer, CouplingAnalyzer, DeadCodeAnalyzer,
        DuplicateCodeAnalyzer, LayerViolationAnalyzer, LengthAnalyzer, NamingAnalyzer,
    },
    wasm,
};

/// Manages plugin registration and lifecycle.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    /// Build registry from config, applying thresholds.
    pub fn from_config(config: &Config, project_dir: &Path) -> Self {
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

        if config.is_enabled("duplicate_code") {
            plugins.push(Box::new(DuplicateCodeAnalyzer));
        }

        if config.is_enabled("coupling") {
            let mut p = CouplingAnalyzer::default();
            if let Some(v) = config.get_usize("coupling", "max_imports") {
                p.max_imports = v;
            }
            plugins.push(Box::new(p));
        }

        if config.is_enabled("naming") {
            let mut p = NamingAnalyzer::default();
            if let Some(v) = config.get_usize("naming", "min_name_length") {
                p.min_name_length = v;
            }
            if let Some(v) = config.get_usize("naming", "max_name_length") {
                p.max_name_length = v;
            }
            plugins.push(Box::new(p));
        }

        if config.is_enabled("dead_code") {
            plugins.push(Box::new(DeadCodeAnalyzer));
        }

        if config.is_enabled("api_surface") {
            let mut p = ApiSurfaceAnalyzer::default();
            if let Some(v) = config.get_usize("api_surface", "max_exported_count") {
                p.max_exported_count = v;
            }
            plugins.push(Box::new(p));
        }

        if config.is_enabled("layer_violation") {
            let p = config
                .get_str("layer_violation", "layers")
                .map(|s| LayerViolationAnalyzer::from_config_str(&s))
                .unwrap_or_default();
            plugins.push(Box::new(p));
        }

        // Load WASM plugins
        let wasm_plugins = wasm::load_wasm_plugins(project_dir);
        for wp in wasm_plugins {
            if config.is_enabled(wp.name()) {
                plugins.push(wp);
            }
        }

        Self { plugins }
    }

    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }
}
