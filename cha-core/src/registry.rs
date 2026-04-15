use std::path::Path;

use crate::{
    Plugin,
    config::Config,
    plugins::{
        ApiSurfaceAnalyzer, BrainMethodAnalyzer, CognitiveComplexityAnalyzer, CommentsAnalyzer,
        ComplexityAnalyzer, CouplingAnalyzer, DataClassAnalyzer, DataClumpsAnalyzer,
        DeadCodeAnalyzer, DesignPatternAdvisor, DivergentChangeAnalyzer, DuplicateCodeAnalyzer,
        ErrorHandlingAnalyzer, FeatureEnvyAnalyzer, GodClassAnalyzer, HardcodedSecretAnalyzer,
        HubLikeDependencyAnalyzer, InappropriateIntimacyAnalyzer, LayerViolationAnalyzer,
        LazyClassAnalyzer, LengthAnalyzer, LongParameterListAnalyzer, MessageChainAnalyzer,
        MiddleManAnalyzer, NamingAnalyzer, PrimitiveObsessionAnalyzer, RefusedBequestAnalyzer,
        ShotgunSurgeryAnalyzer, SpeculativeGeneralityAnalyzer, SwitchStatementAnalyzer,
        TemporaryFieldAnalyzer, TodoTrackerAnalyzer, UnsafeApiAnalyzer,
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

        register_length(&mut plugins, config);
        register_complexity(&mut plugins, config);
        register_simple_plugins(&mut plugins, config);
        register_layer_violation(&mut plugins, config);

        for mut wp in wasm::load_wasm_plugins(project_dir) {
            if config.is_enabled(wp.name()) {
                if let Some(pc) = config.plugins.get(wp.name()) {
                    let opts = pc
                        .options
                        .iter()
                        .filter_map(|(k, v)| {
                            wasm::toml_to_option_value(v).map(|ov| (k.clone(), ov))
                        })
                        .collect();
                    wp.set_options(opts);
                }
                plugins.push(Box::new(wp));
            }
        }

        Self { plugins }
    }

    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    /// Get all plugin names and descriptions from this registry.
    pub fn plugin_info(&self) -> Vec<(String, String)> {
        self.plugins
            .iter()
            .map(|p| (p.name().to_string(), p.description().to_string()))
            .collect()
    }
}

/// Apply a usize config option to a field if present.
fn apply_usize(config: &Config, plugin: &str, key: &str, target: &mut usize) {
    if let Some(v) = config.get_usize(plugin, key) {
        *target = v;
    }
}

/// Generic helper: register a plugin only if enabled.
fn register_if_enabled(
    plugins: &mut Vec<Box<dyn Plugin>>,
    config: &Config,
    name: &str,
    build: impl FnOnce() -> Box<dyn Plugin>,
) {
    if config.is_enabled(name) {
        plugins.push(build());
    }
}

fn register_simple_plugins(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    register_classic_plugins(plugins, config);
    register_smell_plugins(plugins, config);
}

fn register_classic_plugins(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    register_if_enabled(plugins, config, "coupling", || {
        let mut p = CouplingAnalyzer::default();
        apply_usize(config, "coupling", "max_imports", &mut p.max_imports);
        Box::new(p)
    });
    register_if_enabled(plugins, config, "naming", || {
        let mut p = NamingAnalyzer::default();
        apply_usize(config, "naming", "min_name_length", &mut p.min_name_length);
        apply_usize(config, "naming", "max_name_length", &mut p.max_name_length);
        Box::new(p)
    });
    register_if_enabled(plugins, config, "duplicate_code", || {
        Box::new(DuplicateCodeAnalyzer)
    });
    register_if_enabled(plugins, config, "dead_code", || Box::new(DeadCodeAnalyzer));
    register_if_enabled(plugins, config, "api_surface", || {
        let mut p = ApiSurfaceAnalyzer::default();
        apply_usize(
            config,
            "api_surface",
            "max_exported_count",
            &mut p.max_exported_count,
        );
        Box::new(p)
    });
}

fn register_smell_plugins(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    register_if_enabled(plugins, config, "long_parameter_list", || {
        let mut p = LongParameterListAnalyzer::default();
        apply_usize(
            config,
            "long_parameter_list",
            "max_params",
            &mut p.max_params,
        );
        Box::new(p)
    });
    register_if_enabled(plugins, config, "switch_statement", || {
        let mut p = SwitchStatementAnalyzer::default();
        apply_usize(config, "switch_statement", "max_arms", &mut p.max_arms);
        Box::new(p)
    });
    register_if_enabled(plugins, config, "message_chain", || {
        let mut p = MessageChainAnalyzer::default();
        apply_usize(config, "message_chain", "max_depth", &mut p.max_depth);
        Box::new(p)
    });
    register_if_enabled(plugins, config, "primitive_obsession", || {
        Box::new(PrimitiveObsessionAnalyzer::default())
    });
    register_if_enabled(plugins, config, "data_clumps", || {
        Box::new(DataClumpsAnalyzer::default())
    });
    register_if_enabled(plugins, config, "feature_envy", || {
        Box::new(FeatureEnvyAnalyzer::default())
    });
    register_if_enabled(plugins, config, "middle_man", || {
        Box::new(MiddleManAnalyzer::default())
    });
    register_extended_smell_plugins(plugins, config);
}

fn register_extended_smell_plugins(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    register_if_enabled(plugins, config, "comments", || {
        Box::new(CommentsAnalyzer::default())
    });
    register_if_enabled(plugins, config, "lazy_class", || {
        Box::new(LazyClassAnalyzer::default())
    });
    register_if_enabled(plugins, config, "data_class", || {
        Box::new(DataClassAnalyzer::default())
    });
    register_if_enabled(plugins, config, "design_pattern", || {
        Box::new(DesignPatternAdvisor)
    });
    register_if_enabled(plugins, config, "temporary_field", || {
        Box::new(TemporaryFieldAnalyzer::default())
    });
    register_if_enabled(plugins, config, "speculative_generality", || {
        Box::new(SpeculativeGeneralityAnalyzer)
    });
    register_change_preventer_plugins(plugins, config);
    register_advanced_plugins(plugins, config);
}

fn register_change_preventer_plugins(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    register_if_enabled(plugins, config, "refused_bequest", || {
        Box::new(RefusedBequestAnalyzer::default())
    });
    register_if_enabled(plugins, config, "shotgun_surgery", || {
        Box::new(ShotgunSurgeryAnalyzer::default())
    });
    register_if_enabled(plugins, config, "divergent_change", || {
        Box::new(DivergentChangeAnalyzer::default())
    });
    register_if_enabled(plugins, config, "inappropriate_intimacy", || {
        Box::new(InappropriateIntimacyAnalyzer)
    });
    register_if_enabled(plugins, config, "hardcoded_secret", || {
        Box::new(HardcodedSecretAnalyzer)
    });
}

// cha:ignore long_method
fn register_advanced_plugins(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    register_if_enabled(plugins, config, "cognitive_complexity", || {
        let mut p = CognitiveComplexityAnalyzer::default();
        apply_usize(
            config,
            "cognitive_complexity",
            "threshold",
            &mut p.threshold,
        );
        Box::new(p)
    });
    register_if_enabled(plugins, config, "god_class", || {
        let mut p = GodClassAnalyzer::default();
        apply_usize(
            config,
            "god_class",
            "max_external_refs",
            &mut p.max_external_refs,
        );
        apply_usize(config, "god_class", "min_wmc", &mut p.min_wmc);
        Box::new(p)
    });
    register_if_enabled(plugins, config, "brain_method", || {
        let mut p = BrainMethodAnalyzer::default();
        apply_usize(config, "brain_method", "min_lines", &mut p.min_lines);
        apply_usize(
            config,
            "brain_method",
            "min_complexity",
            &mut p.min_complexity,
        );
        Box::new(p)
    });
    register_if_enabled(plugins, config, "hub_like_dependency", || {
        let mut p = HubLikeDependencyAnalyzer::default();
        apply_usize(
            config,
            "hub_like_dependency",
            "max_imports",
            &mut p.max_imports,
        );
        Box::new(p)
    });
    register_if_enabled(plugins, config, "error_handling", || {
        let mut p = ErrorHandlingAnalyzer::default();
        apply_usize(
            config,
            "error_handling",
            "max_unwraps_per_function",
            &mut p.max_unwraps_per_function,
        );
        Box::new(p)
    });
    register_if_enabled(plugins, config, "todo_tracker", || {
        Box::new(TodoTrackerAnalyzer)
    });
    register_if_enabled(plugins, config, "unsafe_api", || {
        Box::new(UnsafeApiAnalyzer)
    });
}

fn register_length(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    if !config.is_enabled("length") {
        return;
    }
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

fn register_complexity(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    if !config.is_enabled("complexity") {
        return;
    }
    let mut p = ComplexityAnalyzer::default();
    if let Some(v) = config.get_usize("complexity", "warn_threshold") {
        p.warn_threshold = v;
    }
    if let Some(v) = config.get_usize("complexity", "error_threshold") {
        p.error_threshold = v;
    }
    plugins.push(Box::new(p));
}

fn register_layer_violation(plugins: &mut Vec<Box<dyn Plugin>>, config: &Config) {
    if !config.is_enabled("layer_violation") {
        return;
    }
    let p = config
        .get_str("layer_violation", "layers")
        .map(|s| LayerViolationAnalyzer::from_config_str(&s))
        .unwrap_or_default();
    plugins.push(Box::new(p));
}
