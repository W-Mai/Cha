/// Extracted function info from AST.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub line_count: usize,
}

/// Extracted class/struct info from AST.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub method_count: usize,
    pub line_count: usize,
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
