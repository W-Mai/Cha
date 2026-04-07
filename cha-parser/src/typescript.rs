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

        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        collect_nodes(root, src, false, &mut functions, &mut classes, &mut imports);

        Some(SourceModel {
            language: "typescript".into(),
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
        match child.kind() {
            "export_statement" => {
                collect_nodes(child, src, true, functions, classes, imports);
            }
            "function_declaration" | "method_definition" => {
                if let Some(mut f) = extract_function(child, src) {
                    f.is_exported = exported;
                    functions.push(f);
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                extract_arrow_functions(child, src, exported, functions);
                collect_nodes(child, src, exported, functions, classes, imports);
            }
            "class_declaration" => {
                if let Some(mut c) = extract_class(child, src) {
                    c.is_exported = exported;
                    classes.push(c);
                }
            }
            "import_statement" => {
                if let Some(i) = extract_import(child, src) {
                    imports.push(i);
                }
            }
            _ => {
                collect_nodes(child, src, false, functions, classes, imports);
            }
        }
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
    let body_hash = node.child_by_field_name("body").map(hash_ast_structure);
    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        complexity: count_complexity(node),
        body_hash,
        is_exported: false,
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
        if child.kind() == "variable_declarator" {
            let name = child
                .child_by_field_name("name")
                .map(|n| node_text(n, src).to_string());
            let value = child.child_by_field_name("value");
            if let (Some(name), Some(value)) = (name, value)
                && value.kind() == "arrow_function"
            {
                let start_line = node.start_position().row + 1;
                let end_line = node.end_position().row + 1;
                let body_hash = value.child_by_field_name("body").map(hash_ast_structure);
                functions.push(FunctionInfo {
                    name,
                    start_line,
                    end_line,
                    line_count: end_line - start_line + 1,
                    complexity: count_complexity(value),
                    body_hash,
                    is_exported: exported,
                });
            }
        }
    }
}

fn extract_class(node: Node, src: &[u8]) -> Option<ClassInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let body = node.child_by_field_name("body")?;
    let mut method_count = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "method_definition" {
            method_count += 1;
        }
    }

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        method_count,
        line_count: end_line - start_line + 1,
        is_exported: false,
    })
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
