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
pub use config::{Config, DebtWeights, LanguageConfig, Strictness, builtin_language_profile};
pub use health::{Grade, HealthScore, score_files};
pub use ignore::filter_ignored;
pub use model::*;
pub use plugin::*;
pub use registry::PluginRegistry;
pub use reporter::{JsonReporter, LlmContextReporter, Reporter, SarifReporter, TerminalReporter};
pub use source::*;

/// Generate JSON Schema for the analysis output (list of findings).
pub fn findings_json_schema() -> String {
    let schema = schemars::schema_for!(Vec<Finding>);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

#[cfg(test)]
mod tests;
