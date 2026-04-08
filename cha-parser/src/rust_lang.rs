use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use cha_core::{ClassInfo, FunctionInfo, ImportInfo, SourceFile, SourceModel};
use tree_sitter::{Node, Parser};

use crate::LanguageParser;

pub struct RustParser;

impl LanguageParser for RustParser {
    fn language_name(&self) -> &str {
        "rust"
    }

    fn parse(&self, file: &SourceFile) -> Option<SourceModel> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .ok()?;
        let tree = parser.parse(&file.content, None)?;
        let root = tree.root_node();
        let src = file.content.as_bytes();

        let mut ctx = ParseContext::new(src);
        ctx.collect_nodes(root, false);

        Some(SourceModel {
            language: "rust".into(),
            total_lines: file.line_count(),
            functions: ctx.col.functions,
            classes: ctx.col.classes,
            imports: ctx.col.imports,
        })
    }
}

/// Accumulator for collected AST items.
struct Collector {
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    imports: Vec<ImportInfo>,
}

/// Bundles source bytes and collector to eliminate repeated parameter passing.
struct ParseContext<'a> {
    src: &'a [u8],
    col: Collector,
}

impl<'a> ParseContext<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self {
            src,
            col: Collector {
                functions: Vec::new(),
                classes: Vec::new(),
                imports: Vec::new(),
            },
        }
    }

    fn collect_nodes(&mut self, node: Node, exported: bool) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_single_node(child, exported);
        }
    }

    fn collect_single_node(&mut self, child: Node, exported: bool) {
        match child.kind() {
            "function_item" => self.push_function(child, exported),
            "impl_item" => self.extract_impl_methods(child),
            "struct_item" | "enum_item" => self.push_struct(child),
            "use_declaration" => self.push_import(child),
            _ => self.collect_nodes(child, false),
        }
    }

    fn push_function(&mut self, node: Node, exported: bool) {
        if let Some(mut f) = extract_function(node, self.src) {
            f.is_exported = exported || has_pub(node);
            self.col.functions.push(f);
        }
    }

    fn push_struct(&mut self, node: Node) {
        if let Some(mut c) = extract_struct(node, self.src) {
            c.is_exported = has_pub(node);
            self.col.classes.push(c);
        }
    }

    fn push_import(&mut self, node: Node) {
        if let Some(i) = extract_use(node, self.src) {
            self.col.imports.push(i);
        }
    }

    fn extract_impl_methods(&mut self, node: Node) {
        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return,
        };
        let impl_name = node
            .child_by_field_name("type")
            .map(|t| node_text(t, self.src).to_string());

        let mut method_count = 0;
        let mut delegating_count = 0;
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item"
                && let Some(mut f) = extract_function(child, self.src)
            {
                f.is_exported = has_pub(child);
                method_count += 1;
                if f.is_delegating {
                    delegating_count += 1;
                }
                self.col.functions.push(f);
            }
        }

        if let Some(ref name) = impl_name
            && let Some(class) = self.col.classes.iter_mut().find(|c| &c.name == name)
        {
            class.method_count += method_count;
            class.delegating_method_count += delegating_count;
        }
    }
}

// Extract and push a function item node.
fn node_text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

fn has_pub(node: Node) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|c| c.kind() == "visibility_modifier")
}

fn hash_ast_structure(node: Node) -> u64 {
    let mut hasher = DefaultHasher::new();
    walk_hash(node, &mut hasher);
    hasher.finish()
}

fn walk_hash(node: Node, hasher: &mut DefaultHasher) {
    node.kind().hash(hasher);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_hash(child, hasher);
    }
}

fn count_complexity(node: Node) -> usize {
    let mut complexity = 1;
    walk_complexity(node, &mut complexity);
    complexity
}

fn walk_complexity(node: Node, count: &mut usize) {
    match node.kind() {
        "if_expression" | "else_clause" | "for_expression" | "while_expression"
        | "loop_expression" | "match_arm" | "closure_expression" => {
            *count += 1;
        }
        "binary_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "&&" || child.kind() == "||" {
                    *count += 1;
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_complexity(child, count);
    }
}

fn extract_function(node: Node, src: &[u8]) -> Option<FunctionInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body");
    let body_hash = body.map(hash_ast_structure);
    let parameter_count = count_parameters(node);
    let parameter_types = extract_param_types(node, src);
    let chain_depth = body.map(max_chain_depth).unwrap_or(0);
    let switch_arms = body.map(count_switch_arms).unwrap_or(0);
    let external_refs = body
        .map(|b| collect_external_refs(b, src))
        .unwrap_or_default();
    let is_delegating = body.map(|b| check_delegating(b, src)).unwrap_or(false);
    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        complexity: count_complexity(node),
        body_hash,
        is_exported: false,
        parameter_count,
        parameter_types,
        chain_depth,
        switch_arms,
        external_refs,
        is_delegating,
    })
}

fn extract_struct(node: Node, src: &[u8]) -> Option<ClassInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    Some(ClassInfo {
        name,
        start_line,
        end_line,
        method_count: 0,
        line_count: end_line - start_line + 1,
        is_exported: false,
        delegating_method_count: 0,
    })
}

fn count_parameters(node: Node) -> usize {
    let params = match node.child_by_field_name("parameters") {
        Some(p) => p,
        None => return 0,
    };
    let mut cursor = params.walk();
    params
        .children(&mut cursor)
        .filter(|c| c.kind() == "parameter" || c.kind() == "self_parameter")
        .count()
}

fn extract_param_types(node: Node, src: &[u8]) -> Vec<String> {
    let params = match node.child_by_field_name("parameters") {
        Some(p) => p,
        None => return vec![],
    };
    let mut types = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() == "parameter"
            && let Some(ty) = child.child_by_field_name("type")
        {
            types.push(normalize_type(node_text(ty, src)));
        }
    }
    types.sort();
    types
}

/// Strip reference/lifetime wrappers to get the canonical type name.
fn normalize_type(raw: &str) -> String {
    raw.trim_start_matches('&')
        .trim_start_matches("mut ")
        .trim()
        .to_string()
}

fn max_chain_depth(node: Node) -> usize {
    let mut max = 0;
    walk_chain_depth(node, &mut max);
    max
}

fn walk_chain_depth(node: Node, max: &mut usize) {
    if node.kind() == "field_expression" {
        let depth = measure_chain(node);
        if depth > *max {
            *max = depth;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_chain_depth(child, max);
    }
}

/// Count consecutive field accesses (a.b.c.d), skipping method calls.
fn measure_chain(node: Node) -> usize {
    let mut depth = 0;
    let mut current = node;
    while current.kind() == "field_expression" {
        depth += 1;
        if let Some(obj) = current.child_by_field_name("value") {
            current = obj;
        } else {
            break;
        }
    }
    depth
}

fn count_switch_arms(node: Node) -> usize {
    let mut count = 0;
    walk_switch_arms(node, &mut count);
    count
}

fn walk_switch_arms(node: Node, count: &mut usize) {
    if node.kind() == "match_arm" {
        *count += 1;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_switch_arms(child, count);
    }
}

fn collect_external_refs(node: Node, src: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    walk_external_refs(node, src, &mut refs);
    refs.sort();
    refs.dedup();
    refs
}

/// Walk a field_expression chain to its root identifier.
fn field_chain_root(node: Node) -> Node {
    let mut current = node;
    while current.kind() == "field_expression" {
        match current.child_by_field_name("value") {
            Some(child) => current = child,
            None => break,
        }
    }
    current
}

fn walk_external_refs(node: Node, src: &[u8], refs: &mut Vec<String>) {
    if node.kind() == "field_expression" {
        // Walk to the root object of the chain (a.b.c → a)
        let root = field_chain_root(node);
        let text = node_text(root, src);
        if text != "self" && !text.is_empty() {
            refs.push(text.to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_external_refs(child, src, refs);
    }
}

/// Extract the single statement from a block body, if exactly one exists.
fn single_stmt(body: Node) -> Option<Node> {
    let mut cursor = body.walk();
    let stmts: Vec<_> = body
        .children(&mut cursor)
        .filter(|c| c.kind() != "{" && c.kind() != "}")
        .collect();
    (stmts.len() == 1).then(|| stmts[0])
}

/// Check if a node is a method call on an external object (not self).
fn is_external_call(node: Node, src: &[u8]) -> bool {
    node.kind() == "call_expression"
        && node.child_by_field_name("function").is_some_and(|func| {
            func.kind() == "field_expression"
                && func
                    .child_by_field_name("value")
                    .is_some_and(|obj| node_text(obj, src) != "self")
        })
}

fn check_delegating(body: Node, src: &[u8]) -> bool {
    let Some(stmt) = single_stmt(body) else {
        return false;
    };
    let expr = match stmt.kind() {
        "expression_statement" => stmt.child(0).unwrap_or(stmt),
        "return_expression" => stmt.child(1).unwrap_or(stmt),
        _ => stmt,
    };
    is_external_call(expr, src)
}

fn extract_use(node: Node, src: &[u8]) -> Option<ImportInfo> {
    let text = node_text(node, src);
    // Extract the path from "use foo::bar::baz;"
    let source = text
        .strip_prefix("use ")?
        .trim_end_matches(';')
        .trim()
        .to_string();
    Some(ImportInfo {
        source,
        line: node.start_position().row + 1,
    })
}
