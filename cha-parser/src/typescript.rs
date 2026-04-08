use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use cha_core::{ClassInfo, FunctionInfo, ImportInfo, SourceFile, SourceModel};
use tree_sitter::{Node, Parser};

use crate::LanguageParser;

pub struct TypeScriptParser;

impl LanguageParser for TypeScriptParser {
    fn language_name(&self) -> &str {
        "typescript"
    }

    fn parse(&self, file: &SourceFile) -> Option<SourceModel> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .ok()?;
        let tree = parser.parse(&file.content, None)?;
        let root = tree.root_node();
        let src = file.content.as_bytes();

        let mut col = Collector {
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
        };

        collect_nodes(root, src, false, &mut col);

        Some(SourceModel {
            language: "typescript".into(),
            total_lines: file.line_count(),
            functions: col.functions,
            classes: col.classes,
            imports: col.imports,
        })
    }
}

/// Accumulator for collected AST items.
struct Collector {
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    imports: Vec<ImportInfo>,
}

fn collect_nodes(node: Node, src: &[u8], exported: bool, col: &mut Collector) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_single_node(child, src, exported, col);
    }
}

fn collect_single_node(child: Node, src: &[u8], exported: bool, col: &mut Collector) {
    match child.kind() {
        "export_statement" => collect_nodes(child, src, true, col),
        "function_declaration" | "method_definition" => push_function(child, src, exported, col),
        "lexical_declaration" | "variable_declaration" => {
            extract_arrow_functions(child, src, exported, &mut col.functions);
            collect_nodes(child, src, exported, col);
        }
        "class_declaration" => push_class(child, src, exported, col),
        "import_statement" => push_import(child, src, col),
        _ => collect_nodes(child, src, false, col),
    }
}

fn push_function(node: Node, src: &[u8], exported: bool, col: &mut Collector) {
    if let Some(mut f) = extract_function(node, src) {
        f.is_exported = exported;
        col.functions.push(f);
    }
}

fn push_class(node: Node, src: &[u8], exported: bool, col: &mut Collector) {
    if let Some(mut c) = extract_class(node, src) {
        c.is_exported = exported;
        col.classes.push(c);
    }
}

fn push_import(node: Node, src: &[u8], col: &mut Collector) {
    if let Some(i) = extract_import(node, src) {
        col.imports.push(i);
    }
}

fn node_text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

/// Hash the AST structure of a node (kind + children structure, ignoring names).
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

fn extract_arrow_functions(
    node: Node,
    src: &[u8],
    exported: bool,
    functions: &mut Vec<FunctionInfo>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator"
            && let Some(f) = try_extract_arrow(child, node, src, exported)
        {
            functions.push(f);
        }
    }
}

// Try to extract an arrow function from a variable declarator.
fn try_extract_arrow(child: Node, decl: Node, src: &[u8], exported: bool) -> Option<FunctionInfo> {
    let name = child
        .child_by_field_name("name")
        .map(|n| node_text(n, src).to_string())?;
    let value = child.child_by_field_name("value")?;
    if value.kind() != "arrow_function" {
        return None;
    }
    let start_line = decl.start_position().row + 1;
    let end_line = decl.end_position().row + 1;
    let body = value.child_by_field_name("body");
    let body_hash = body.map(hash_ast_structure);
    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        complexity: count_complexity(value),
        body_hash,
        is_exported: exported,
        parameter_count: count_parameters(value),
        parameter_types: extract_param_types(value, src),
        chain_depth: body.map(max_chain_depth).unwrap_or(0),
        switch_arms: body.map(count_switch_arms).unwrap_or(0),
        external_refs: body
            .map(|b| collect_external_refs(b, src))
            .unwrap_or_default(),
        is_delegating: body.map(|b| check_delegating(b, src)).unwrap_or(false),
    })
}

fn extract_class(node: Node, src: &[u8]) -> Option<ClassInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let body = node.child_by_field_name("body")?;
    let mut method_count = 0;
    let mut delegating_method_count = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "method_definition" {
            method_count += 1;
            if let Some(method_body) = child.child_by_field_name("body")
                && check_delegating(method_body, src)
            {
                delegating_method_count += 1;
            }
        }
    }

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        method_count,
        line_count: end_line - start_line + 1,
        is_exported: false,
        delegating_method_count,
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
        .filter(|c| {
            matches!(
                c.kind(),
                "required_parameter" | "optional_parameter" | "rest_parameter"
            )
        })
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
        if let Some(ann) = child.child_by_field_name("type") {
            types.push(node_text(ann, src).to_string());
        }
    }
    types.sort();
    types
}

fn max_chain_depth(node: Node) -> usize {
    let mut max = 0;
    walk_chain_depth(node, &mut max);
    max
}

fn walk_chain_depth(node: Node, max: &mut usize) {
    if node.kind() == "member_expression" {
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

/// Count consecutive property accesses (a.b.c.d), skipping method calls.
fn measure_chain(node: Node) -> usize {
    let mut depth = 0;
    let mut current = node;
    while current.kind() == "member_expression" {
        depth += 1;
        if let Some(obj) = current.child_by_field_name("object") {
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
    if node.kind() == "switch_case" || node.kind() == "switch_default" {
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

fn member_chain_root(node: Node) -> Node {
    let mut current = node;
    while current.kind() == "member_expression" {
        match current.child_by_field_name("object") {
            Some(child) => current = child,
            None => break,
        }
    }
    current
}

fn walk_external_refs(node: Node, src: &[u8], refs: &mut Vec<String>) {
    if node.kind() == "member_expression" {
        let root = member_chain_root(node);
        let text = node_text(root, src);
        if text != "this" && text != "self" && !text.is_empty() {
            refs.push(text.to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_external_refs(child, src, refs);
    }
}

fn single_stmt(body: Node) -> Option<Node> {
    let mut cursor = body.walk();
    let stmts: Vec<_> = body
        .children(&mut cursor)
        .filter(|c| c.kind() != "{" && c.kind() != "}")
        .collect();
    (stmts.len() == 1).then(|| stmts[0])
}

fn is_external_call(node: Node, src: &[u8]) -> bool {
    node.kind() == "call_expression"
        && node.child_by_field_name("function").is_some_and(|func| {
            func.kind() == "member_expression"
                && func
                    .child_by_field_name("object")
                    .is_some_and(|obj| node_text(obj, src) != "this")
        })
}

fn check_delegating(body: Node, src: &[u8]) -> bool {
    let Some(stmt) = single_stmt(body) else {
        return false;
    };
    let expr = match stmt.kind() {
        "return_statement" => stmt.child(1).unwrap_or(stmt),
        "expression_statement" => stmt.child(0).unwrap_or(stmt),
        _ => stmt,
    };
    is_external_call(expr, src)
}

fn count_complexity(node: Node) -> usize {
    let mut complexity = 1;
    walk_complexity(node, &mut complexity);
    complexity
}

fn walk_complexity(node: Node, count: &mut usize) {
    match node.kind() {
        "if_statement" | "else_clause" | "for_statement" | "for_in_statement"
        | "while_statement" | "do_statement" | "switch_case" | "catch_clause"
        | "ternary_expression" => {
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

fn extract_import(node: Node, src: &[u8]) -> Option<ImportInfo> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string" {
            let raw = node_text(child, src);
            let source = raw.trim_matches(|c| c == '\'' || c == '"').to_string();
            return Some(ImportInfo {
                source,
                line: node.start_position().row + 1,
            });
        }
    }
    None
}
