use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{SourceFile, SourceModel};

/// Severity level for a finding.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Hint,
    Warning,
    Error,
}

/// Smell category from refactoring literature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SmellCategory {
    Bloaters,
    OoAbusers,
    ChangePreventers,
    Dispensables,
    Couplers,
}

/// Source location of a finding.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Location {
    pub path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub name: Option<String>,
}

/// A single analysis finding.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Finding {
    pub smell_name: String,
    pub category: SmellCategory,
    pub severity: Severity,
    pub location: Location,
    pub message: String,
    pub suggested_refactorings: Vec<String>,
}

/// Analysis context passed to plugins.
pub struct AnalysisContext<'a> {
    pub file: &'a SourceFile,
    pub model: &'a SourceModel,
}

/// Core trait that all analyzers implement.
pub trait Plugin: Send + Sync {
    /// Unique identifier for this plugin.
    fn name(&self) -> &str;

    /// Run analysis on a single file and return findings.
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding>;
}
