use cha_core::SourceFile;
use tree_sitter::{Node, Parser};

use crate::{ClassInfo, FunctionInfo, ImportInfo, LanguageParser, SourceModel};

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

        collect_nodes(root, src, &mut functions, &mut classes, &mut imports);

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
    functions: &mut Vec<FunctionInfo>,
    classes: &mut Vec<ClassInfo>,
    imports: &mut Vec<ImportInfo>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "method_definition" => {
                if let Some(f) = extract_function(child, src) {
                    functions.push(f);
                }
            }
            // Arrow functions assigned to variables: const foo = (...) => { ... }
            "lexical_declaration" | "variable_declaration" => {
                extract_arrow_functions(child, src, functions);
                // Recurse for nested structures
                collect_nodes(child, src, functions, classes, imports);
            }
            "class_declaration" => {
                if let Some(c) = extract_class(child, src) {
                    classes.push(c);
                }
            }
            "import_statement" => {
                if let Some(i) = extract_import(child, src) {
                    imports.push(i);
                }
            }
            _ => {
                collect_nodes(child, src, functions, classes, imports);
            }
        }
    }
}

fn node_text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

fn extract_function(node: Node, src: &[u8]) -> Option<FunctionInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
    })
}

fn extract_arrow_functions(node: Node, src: &[u8], functions: &mut Vec<FunctionInfo>) {
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
                functions.push(FunctionInfo {
                    name,
                    start_line,
                    end_line,
                    line_count: end_line - start_line + 1,
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
    })
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
