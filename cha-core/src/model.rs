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
    /// Field names checked for null/None in this function (for Null Object pattern).
    pub null_check_fields: Vec<String>,
    /// The field/variable name being dispatched on in switch/match (for Strategy/State).
    pub switch_dispatch_target: Option<String>,
    /// Number of optional parameters (for Builder pattern).
    pub optional_param_count: usize,
    /// Names of functions/methods called in this function body (for call graph).
    pub called_functions: Vec<String>,
    /// Cognitive complexity score [SonarSource 2017] — nesting-aware understandability metric.
    pub cognitive_complexity: usize,
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
    /// Field types (parallel to field_names).
    pub field_types: Vec<String>,
    /// Whether the class has non-accessor methods (business logic).
    pub has_behavior: bool,
    /// Whether this is an interface or abstract class.
    pub is_interface: bool,
    /// Parent class/trait name (for Refused Bequest).
    pub parent_name: Option<String>,
    /// Number of overridden methods (for Refused Bequest).
    pub override_count: usize,
    /// Number of self-method calls in the longest method (for Template Method).
    pub self_call_count: usize,
    /// Whether the class has a listener/callback collection field.
    pub has_listener_field: bool,
    /// Whether the class has a notify/emit method.
    pub has_notify_method: bool,
}

/// Extracted import info.
#[derive(Debug, Clone, Default)]
pub struct ImportInfo {
    pub source: String,
    pub line: usize,
    /// True for module declarations (e.g. Rust `mod foo;`).
    pub is_module_decl: bool,
}

/// A comment extracted from source code by the language parser.
#[derive(Debug, Clone)]
pub struct CommentInfo {
    pub text: String,
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
    pub comments: Vec<CommentInfo>,
    /// Type aliases: (alias, original). e.g. typedef, using, type =
    pub type_aliases: Vec<(String, String)>,
}
