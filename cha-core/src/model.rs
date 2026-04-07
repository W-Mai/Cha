/// Extracted function info from AST.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub line_count: usize,
    /// Cyclomatic complexity (1 + number of branch points).
    pub complexity: usize,
    /// Hash of the function body AST structure for duplicate detection.
    pub body_hash: Option<u64>,
    /// Whether this function is exported (pub/export).
    pub is_exported: bool,
}

/// Extracted class/struct info from AST.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub method_count: usize,
    pub line_count: usize,
    /// Whether this class is exported.
    pub is_exported: bool,
}

/// Extracted import info.
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub source: String,
    pub line: usize,
}

/// Unified source model produced by parsing.
#[derive(Debug, Clone)]
pub struct SourceModel {
    pub language: String,
    pub total_lines: usize,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
}
