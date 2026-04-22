use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{SourceFile, SourceModel};

/// Severity level for a finding.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    #[default]
    Hint,
    Warning,
    Error,
}

/// Smell category from refactoring literature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SmellCategory {
    #[default]
    Bloaters,
    OoAbusers,
    ChangePreventers,
    Dispensables,
    Couplers,
    Security,
}

/// Source location of a finding.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Location {
    pub path: PathBuf,
    pub start_line: usize,
    /// 0-based column of the start position.
    #[serde(default, skip_serializing_if = "crate::is_zero_usize")]
    pub start_col: usize,
    pub end_line: usize,
    /// 0-based column of the end position.
    #[serde(default, skip_serializing_if = "crate::is_zero_usize")]
    pub end_col: usize,
    pub name: Option<String>,
}

/// A single analysis finding.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Finding {
    pub smell_name: String,
    pub category: SmellCategory,
    pub severity: Severity,
    pub location: Location,
    pub message: String,
    pub suggested_refactorings: Vec<String>,
    /// The actual measured value (e.g. line count, complexity score).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_value: Option<f64>,
    /// The threshold that was exceeded to produce this finding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
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

    /// Plugin version (e.g. "1.0.0").
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    /// Short description of what the plugin detects.
    fn description(&self) -> &str {
        ""
    }

    /// List of authors.
    fn authors(&self) -> Vec<String> {
        vec![env!("CARGO_PKG_AUTHORS").to_string()]
    }

    /// Run analysis on a single file and return findings.
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding>;
}

/// Build a Location pointing at a function's name identifier.
pub fn func_location(path: &std::path::Path, f: &crate::FunctionInfo) -> Location {
    Location {
        path: path.into(),
        start_line: f.start_line,
        start_col: f.name_col,
        end_line: f.start_line,
        end_col: f.name_end_col,
        name: Some(f.name.clone()),
    }
}

/// Build a Location pointing at a class/struct's name identifier.
pub fn class_location(path: &std::path::Path, c: &crate::ClassInfo) -> Location {
    Location {
        path: path.into(),
        start_line: c.start_line,
        start_col: c.name_col,
        end_line: c.start_line,
        end_col: c.name_end_col,
        name: Some(c.name.clone()),
    }
}
