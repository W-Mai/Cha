/// Where a referenced type is defined, from the perspective of the file that
/// uses it. Used by abstraction-boundary analyses to distinguish "own domain"
/// types from "pulled in from a library" types.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "module")]
pub enum TypeOrigin {
    /// Declared inside the project (resolved via project-wide type registry
    /// or an import pointing at a project-local path).
    Local,
    /// Imported from an external module / crate / package. Carries the module
    /// name if known (Rust crate path root, Go module path, npm package name,
    /// C header filename without extension). May be empty if only structure
    /// says "external" (e.g. `#include <...>` without the header name).
    External(String),
    /// Built-in primitive / standard library scalar (int, bool, &str, char…).
    Primitive,
    /// Could not be resolved. Detection treats this as potentially external
    /// but with lower confidence.
    #[default]
    Unknown,
}

/// A single arm value of a `switch`/`match` construct. Recorded so
/// signature-based analyses can notice "switch on string constants" or
/// "switch on integer magic numbers" dispatch patterns. Non-literal
/// patterns (Rust enum variants, Python capture patterns, `default`)
/// collapse to `Other`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArmValue {
    Str(String),
    Int(i64),
    Char(char),
    /// Non-literal pattern — enum variants, destructuring, guards,
    /// `default`, `_`. Kept counted so callers can tell "dispatch with
    /// N arms total" from "dispatch with N literal arms".
    Other,
}

/// A function parameter's (or return value's) type, with resolved origin.
/// Produced by parsers after combining AST type text with the file's imports.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct TypeRef {
    /// Innermost identifier after stripping references, generics, containers.
    /// e.g. `&mut Vec<tree_sitter::Node>` → `"Node"`.
    pub name: String,
    /// Original source text as written, for messages and debugging.
    /// e.g. `"&mut Vec<tree_sitter::Node>"`.
    pub raw: String,
    /// Where the type is declared.
    pub origin: TypeOrigin,
}

/// Extracted function info from AST.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// 0-based column of the function name identifier.
    pub name_col: usize,
    /// 0-based end column of the function name identifier.
    pub name_end_col: usize,
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
    /// Arm values of each `switch`/`match` arm in the function body,
    /// in source order, across *all* dispatch constructs. Enables
    /// `stringly_typed_dispatch` to notice "≥ 3 arms are string
    /// literals" without re-walking the AST. Empty for functions
    /// with no switch/match.
    pub switch_arm_values: Vec<ArmValue>,
    /// Whether this function only delegates to another object's method (for Middle Man).
    pub is_delegating: bool,
    /// Parameter types **in declaration order**, each resolved to a TypeRef.
    /// Preserves position (first param = index 0) so positional analyses work.
    pub parameter_types: Vec<TypeRef>,
    /// Parameter identifier names, parallel to `parameter_types`. Empty
    /// string for anonymous parameters (C `void foo(int);`). Drives
    /// name-semantic analyses like `primitive_representation`.
    pub parameter_names: Vec<String>,
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
    /// Declared return type (None if not annotated or inferred), resolved the
    /// same way as parameter types. Drives return_type_leak detection.
    pub return_type: Option<TypeRef>,
}

/// Extracted class/struct info from AST.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// 0-based column of the class/struct name identifier.
    pub name_col: usize,
    /// 0-based end column of the class/struct name identifier.
    pub name_end_col: usize,
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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImportInfo {
    pub source: String,
    pub line: usize,
    /// 0-based column of the import statement.
    pub col: usize,
    /// True for module declarations (e.g. Rust `mod foo;`).
    pub is_module_decl: bool,
}

/// A comment extracted from source code by the language parser.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommentInfo {
    pub text: String,
    pub line: usize,
}

/// Unified source model produced by parsing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// Compact structural summary of a file — symbol-level view without the
/// per-function-body detail that analyze plugins need. Serves `cha deps`,
/// future LSP workspace-symbols, and anywhere a reader needs "what
/// classes/functions live here and how are they related" without caring
/// about complexity metrics or TypeRef origin resolution.
///
/// One-way derivable from `SourceModel`; cached separately so light
/// consumers don't pay `SourceModel`'s deserialise cost.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolIndex {
    pub language: String,
    pub total_lines: usize,
    pub imports: Vec<ImportInfo>,
    pub classes: Vec<ClassSymbol>,
    pub functions: Vec<FunctionSymbol>,
    /// `(alias, original)`. Mirrors `SourceModel.type_aliases`.
    pub type_aliases: Vec<(String, String)>,
}

/// Symbol-level view of a class — everything deps/LSP/hotspot need to
/// reason about a class without parsing method bodies. Fields intentionally
/// track the subset of `ClassInfo` that survives cross-file consumption.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassSymbol {
    pub name: String,
    pub parent_name: Option<String>,
    pub is_interface: bool,
    pub is_exported: bool,
    pub method_count: usize,
    pub has_behavior: bool,
    pub field_names: Vec<String>,
    pub field_types: Vec<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub name_col: usize,
    pub name_end_col: usize,
}

/// Symbol-level view of a function — name + signature + call-graph input.
/// Omits body_hash, complexity, cognitive, external_refs, chain_depth,
/// parameter_types (TypeRef), return_type — those live in `FunctionInfo`
/// for analyze plugins.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionSymbol {
    pub name: String,
    pub is_exported: bool,
    pub parameter_count: usize,
    pub called_functions: Vec<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub name_col: usize,
    pub name_end_col: usize,
    /// Bare type names (no module prefix, no origin info) for each
    /// parameter in declaration order. Sufficient for signature-based
    /// clustering (C OOP attribution, call-graph refinement) without
    /// pulling in TypeRef's origin resolution, which is analyze-only.
    pub parameter_type_names: Vec<String>,
    /// Parameter identifier names, parallel to `parameter_type_names`.
    /// Empty string for anonymous params (C `void foo(int);`). Enables
    /// name-semantic views (e.g. LSP hover "email: String") without
    /// loading the full `SourceModel`.
    pub parameter_names: Vec<String>,
    /// Bare return type name (same conventions as parameter_type_names);
    /// `None` if the function has no declared return type.
    pub return_type_name: Option<String>,
    /// Mirror of `FunctionInfo.switch_arm_values`. Enables LSP / summary
    /// tools to recognise stringly-typed dispatchers without loading
    /// the full source model.
    pub switch_arm_values: Vec<ArmValue>,
}

impl SymbolIndex {
    /// Project a `SourceModel` onto the symbol-level view. Cheap —
    /// clones strings but no heavy structures.
    pub fn from_source_model(m: &SourceModel) -> Self {
        Self {
            language: m.language.clone(),
            total_lines: m.total_lines,
            imports: m.imports.clone(),
            classes: m.classes.iter().map(ClassSymbol::from_class_info).collect(),
            functions: m
                .functions
                .iter()
                .map(FunctionSymbol::from_function_info)
                .collect(),
            type_aliases: m.type_aliases.clone(),
        }
    }
}

impl ClassSymbol {
    pub fn from_class_info(c: &ClassInfo) -> Self {
        Self {
            name: c.name.clone(),
            parent_name: c.parent_name.clone(),
            is_interface: c.is_interface,
            is_exported: c.is_exported,
            method_count: c.method_count,
            has_behavior: c.has_behavior,
            field_names: c.field_names.clone(),
            field_types: c.field_types.clone(),
            start_line: c.start_line,
            end_line: c.end_line,
            name_col: c.name_col,
            name_end_col: c.name_end_col,
        }
    }
}

impl FunctionSymbol {
    pub fn from_function_info(f: &FunctionInfo) -> Self {
        Self {
            name: f.name.clone(),
            is_exported: f.is_exported,
            parameter_count: f.parameter_count,
            called_functions: f.called_functions.clone(),
            start_line: f.start_line,
            end_line: f.end_line,
            name_col: f.name_col,
            name_end_col: f.name_end_col,
            parameter_type_names: f.parameter_types.iter().map(|t| t.raw.clone()).collect(),
            parameter_names: f.parameter_names.clone(),
            return_type_name: f.return_type.as_ref().map(|t| t.raw.clone()),
            switch_arm_values: f.switch_arm_values.clone(),
        }
    }
}
