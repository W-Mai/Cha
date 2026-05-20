use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::{FunctionInfo, SourceFile, SourceModel, TypeRef};

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
    /// Composite priority score: severity × overshoot × hotspot factor.
    /// Populated by `prioritize_findings` after analysis completes; absent
    /// for findings produced but not yet ranked (pre-sort).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<f64>,
}

/// Project-level queries available to all plugins (built-in + WASM).
///
/// Methods return owned/cheap-to-copy types (`PathBuf`/`Vec`) so WASM host
/// imports can wrap them directly without borrow gymnastics. For built-in
/// plugins needing bulk access (iterate every model), use the
/// `ProjectQueryBulk` extension trait via downcast.
pub trait ProjectQuery: Send + Sync {
    // === Reference relations ===

    /// True if `name` is called from any file other than `exclude_path`.
    fn is_called_externally(&self, name: &str, exclude_path: &Path) -> bool;

    /// All files that reference `name` (excluding self-references).
    fn callers_of(&self, name: &str) -> Vec<PathBuf>;

    /// Pre-computed cross-file call counts: `((caller, callee), count)`.
    fn cross_file_call_counts(&self) -> Vec<((PathBuf, PathBuf), u32)>;

    // === Symbol location ===

    /// First file that declared this function.
    fn function_home(&self, name: &str) -> Option<PathBuf>;

    /// First `(file, FunctionInfo)` tuple — fuller than `function_home`.
    fn function_by_name(&self, name: &str) -> Option<(PathBuf, FunctionInfo)>;

    /// First file that declared this class/struct.
    fn class_home(&self, name: &str) -> Option<PathBuf>;

    /// O(1) model lookup by path.
    fn model_by_path(&self, path: &Path) -> Option<SourceModel>;

    // === Type system ===

    /// True if `name` is declared somewhere in the project.
    fn is_project_type(&self, name: &str) -> bool;

    /// True if the type is a genuine third-party dependency
    /// (External origin AND not stdlib AND not workspace sibling).
    fn is_third_party(&self, type_ref: &TypeRef) -> bool;

    /// Workspace sibling crate names (Rust workspace) — empty otherwise.
    fn workspace_crate_names(&self) -> Vec<String>;

    // === Path shape ===

    /// True if path looks like a test file or sits inside a test directory.
    fn is_test_path(&self, path: &Path) -> bool;

    // === Project metadata ===

    /// Total count of analyzed files.
    fn file_count(&self) -> usize;
}

/// Bulk access for in-process plugins. WASM plugins cannot reach this trait —
/// they're stuck with point queries from `ProjectQuery`.
pub trait ProjectQueryBulk: ProjectQuery {
    fn iter_models(&self) -> Box<dyn Iterator<Item = (&Path, &SourceModel)> + '_>;
}

/// Analysis context passed to plugins.
pub struct AnalysisContext<'a> {
    pub file: &'a SourceFile,
    pub model: &'a SourceModel,
    pub tree: Option<&'a tree_sitter::Tree>,
    pub ts_language: Option<&'a tree_sitter::Language>,
    /// Project-level query interface. Wrapped in Arc so WASM hosts can take
    /// owned ownership for store lifetime; built-in plugins deref through `&`.
    pub project: Option<&'a std::sync::Arc<dyn ProjectQuery>>,
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

    /// Smell names this plugin can produce.
    /// Used by the host for smell-level filtering, docs, and `cha plugin list`.
    /// Default is empty — plugins should override to declare their smells.
    fn smells(&self) -> Vec<String> {
        Vec::new()
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
