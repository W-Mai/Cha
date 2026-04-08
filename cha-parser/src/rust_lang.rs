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

        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        collect_nodes(root, src, false, &mut functions, &mut classes, &mut imports);

        Some(SourceModel {
            language: "rust".into(),
            total_lines: file.line_count(),
            functions,
            classes,
            imports,
        })
    }
}

fn collect_nodes(
    node: Node,
    src: &[u8],
    exported: bool,
    functions: &mut Vec<FunctionInfo>,
    classes: &mut Vec<ClassInfo>,
    imports: &mut Vec<ImportInfo>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_single_node(child, src, exported, functions, classes, imports);
    }
}

// Dispatch a single child node by kind.
fn collect_single_node(
    child: Node,
    src: &[u8],
    exported: bool,
    functions: &mut Vec<FunctionInfo>,
    classes: &mut Vec<ClassInfo>,
    imports: &mut Vec<ImportInfo>,
) {
    match child.kind() {
        "function_item" => {
            collect_function_node(child, src, exported, functions);
        }
        "impl_item" => {
            extract_impl_methods(child, src, functions);
        }
        "struct_item" | "enum_item" => {
            collect_struct_node(child, src, classes);
        }
        "use_declaration" => {
            if let Some(i) = extract_use(child, src) {
                imports.push(i);
            }
        }
        _ => {
            collect_nodes(child, src, false, functions, classes, imports);
        }
    }
}

// Extract and push a function item node.
fn collect_function_node(
    node: Node,
    src: &[u8],
    exported: bool,
    functions: &mut Vec<FunctionInfo>,
) {
    if let Some(mut f) = extract_function(node, src) {
        f.is_exported = exported || has_pub(node);
        functions.push(f);
    }
}

// Extract and push a struct/enum node.
fn collect_struct_node(node: Node, src: &[u8], classes: &mut Vec<ClassInfo>) {
    if let Some(mut c) = extract_struct(node, src) {
        c.is_exported = has_pub(node);
        classes.push(c);
    }
}

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

fn extract_impl_methods(node: Node, src: &[u8], functions: &mut Vec<FunctionInfo>) {
    let body = match node.child_by_field_name("body") {
        Some(b) => b,
        None => return,
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "function_item"
            && let Some(mut f) = extract_function(child, src)
        {
            f.is_exported = has_pub(child);
            functions.push(f);
        }
    }
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
            types.push(node_text(ty, src).to_string());
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
    if node.kind() == "call_expression" || node.kind() == "field_expression" {
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

fn measure_chain(node: Node) -> usize {
    let mut depth = 0;
    let mut current = node;
    while current.kind() == "field_expression" || current.kind() == "call_expression" {
        depth += 1;
        if let Some(obj) = current
            .child_by_field_name("value")
            .or(current.child_by_field_name("function"))
        {
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

fn walk_external_refs(node: Node, src: &[u8], refs: &mut Vec<String>) {
    if node.kind() == "field_expression"
        && let Some(obj) = node.child_by_field_name("value")
    {
        let text = node_text(obj, src);
        if text != "self" && !text.is_empty() {
            refs.push(text.to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_external_refs(child, src, refs);
    }
}

fn check_delegating(body: Node, src: &[u8]) -> bool {
    let mut cursor = body.walk();
    let stmts: Vec<_> = body
        .children(&mut cursor)
        .filter(|c| c.kind() != "{" && c.kind() != "}")
        .collect();
    if stmts.len() != 1 {
        return false;
    }
    let stmt = stmts[0];
    let expr = if stmt.kind() == "expression_statement" {
        stmt.child(0).unwrap_or(stmt)
    } else {
        stmt
    };
    expr.kind() == "call_expression"
        && expr.child_by_field_name("function").is_some_and(|func| {
            func.kind() == "field_expression"
                && func
                    .child_by_field_name("value")
                    .is_some_and(|obj| node_text(obj, src) != "self")
        })
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
