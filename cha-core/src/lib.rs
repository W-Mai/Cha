mod baseline;
mod cache;
pub mod config;
pub mod graph;
mod health;
pub mod html_reporter;
mod ignore;
mod model;
mod plugin;
pub mod plugins;
mod registry;
pub mod reporter;
mod source;
pub mod wasm;

pub use baseline::Baseline;
pub use cache::AnalysisCache;
pub use config::{
    Config, DebtWeights, LanguageConfig, LayersConfig, Strictness, TierConfig,
    builtin_language_profile,
};
pub use health::{Grade, HealthScore, score_files};
pub use ignore::filter_ignored;
pub use model::*;
pub use plugin::*;
pub use registry::PluginRegistry;
pub use reporter::{JsonReporter, LlmContextReporter, Reporter, SarifReporter, TerminalReporter};
pub use source::*;

/// Helper for serde skip_serializing_if on f64 fields.
pub fn is_zero_f64(v: &f64) -> bool {
    *v == 0.0
}

/// Sort findings by priority descending (most important first).
/// priority = severity_weight × overshoot × compound_factor
pub fn prioritize_findings(findings: &mut [Finding]) {
    let per_file: std::collections::HashMap<std::path::PathBuf, usize> = {
        let mut m = std::collections::HashMap::new();
        for f in findings.iter() {
            *m.entry(f.location.path.clone()).or_default() += 1;
        }
        m
    };
    findings.sort_by(|a, b| {
        let pa = finding_priority(a, &per_file);
        let pb = finding_priority(b, &per_file);
        pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn finding_priority(
    f: &Finding,
    per_file: &std::collections::HashMap<std::path::PathBuf, usize>,
) -> f64 {
    let sev = match f.severity {
        Severity::Error => 3.0,
        Severity::Warning => 2.0,
        Severity::Hint => 1.0,
    };
    let overshoot = match (f.actual_value, f.threshold) {
        (Some(a), Some(t)) if t > 0.0 => (a / t).max(1.0),
        _ => 1.0,
    };
    let compound = if *per_file.get(&f.location.path).unwrap_or(&1) > 3 {
        1.5
    } else {
        1.0
    };
    sev * overshoot * compound
}

/// Generate JSON Schema for the analysis output (list of findings).
pub fn findings_json_schema() -> String {
    let schema = schemars::schema_for!(Vec<Finding>);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

#[cfg(test)]
mod tests;
