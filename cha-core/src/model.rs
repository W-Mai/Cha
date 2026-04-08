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
    /// Number of parameters.
    pub parameter_count: usize,
    /// Names of external identifiers referenced in the body (for Feature Envy).
    pub external_refs: Vec<String>,
    /// Max method chain depth in the body (for Message Chains).
    pub chain_depth: usize,
    /// Number of switch/match arms (for Switch Statements).
    pub switch_arms: usize,
    /// Whether this function only delegates to another object's method (for Middle Man).
    pub is_delegating: bool,
    /// Sorted parameter type names (for Data Clumps / Primitive Obsession).
    pub parameter_types: Vec<String>,
    /// Number of comment lines in the function body.
    pub comment_lines: usize,
    /// Field names referenced in this function body (for Temporary Field).
    pub referenced_fields: Vec<String>,
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
    /// Number of methods that only delegate to another object.
    pub delegating_method_count: usize,
    /// Number of fields/properties.
    pub field_count: usize,
    /// Field names declared in this class.
    pub field_names: Vec<String>,
    /// Whether the class has non-accessor methods (business logic).
    pub has_behavior: bool,
    /// Whether this is an interface or abstract class.
    pub is_interface: bool,
    /// Parent class/trait name (for Refused Bequest).
    pub parent_name: Option<String>,
    /// Number of overridden methods (for Refused Bequest).
    pub override_count: usize,
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
