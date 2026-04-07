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
        match child.kind() {
            "function_item" => {
                if let Some(mut f) = extract_function(child, src) {
                    f.is_exported = exported || has_pub(child);
                    functions.push(f);
                }
            }
            "impl_item" => {
                extract_impl_methods(child, src, functions);
            }
            "struct_item" | "enum_item" => {
                if let Some(mut c) = extract_struct(child, src) {
                    c.is_exported = has_pub(child);
                    classes.push(c);
                }
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
