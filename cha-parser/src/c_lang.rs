use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use cha_core::{ClassInfo, FunctionInfo, ImportInfo, SourceFile, SourceModel};
use tree_sitter::{Node, Parser};

use crate::LanguageParser;

pub struct CParser;
pub struct CppParser;

impl LanguageParser for CParser {
    fn language_name(&self) -> &str {
        "c"
    }
    fn parse(&self, file: &SourceFile) -> Option<SourceModel> {
        parse_c_like(file, "c", &tree_sitter_c::LANGUAGE.into())
    }
}

impl LanguageParser for CppParser {
    fn language_name(&self) -> &str {
        "cpp"
    }
    fn parse(&self, file: &SourceFile) -> Option<SourceModel> {
        parse_c_like(file, "cpp", &tree_sitter_cpp::LANGUAGE.into())
    }
}

fn parse_c_like(
    file: &SourceFile,
    lang: &str,
    language: &tree_sitter::Language,
) -> Option<SourceModel> {
    let mut parser = Parser::new();
    parser.set_language(language).ok()?;
    let tree = parser.parse(&file.content, None)?;
    let root = tree.root_node();
    let src = file.content.as_bytes();

    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();

    collect_top_level(root, src, &mut functions, &mut classes, &mut imports);

    Some(SourceModel {
        language: lang.into(),
        total_lines: file.line_count(),
        functions,
        classes,
        imports,
    })
}

fn collect_top_level(
    root: Node,
    src: &[u8],
    functions: &mut Vec<FunctionInfo>,
    classes: &mut Vec<ClassInfo>,
    imports: &mut Vec<ImportInfo>,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(f) = extract_function(child, src) {
                    functions.push(f);
                }
            }
            "struct_specifier" | "class_specifier" => {
                if let Some(c) = extract_class(child, src) {
                    classes.push(c);
                }
            }
            "preproc_include" => {
                if let Some(imp) = extract_include(child, src) {
                    imports.push(imp);
                }
            }
            _ => {}
        }
    }
}

fn extract_function(node: Node, src: &[u8]) -> Option<FunctionInfo> {
    let declarator = node.child_by_field_name("declarator")?;
    let name = find_func_name(declarator, src)?.to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body");
    let (param_count, param_types) = extract_params(declarator, src);

    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        complexity: count_complexity(node),
        body_hash: body.map(hash_ast),
        is_exported: true,
        parameter_count: param_count,
        parameter_types: param_types,
        chain_depth: body.map(max_chain_depth).unwrap_or(0),
        switch_arms: body.map(count_case_labels).unwrap_or(0),
        external_refs: Vec::new(), // TODO(parser): extract external refs for C/C++
        is_delegating: false,      // TODO(parser): detect delegating for C/C++
        comment_lines: count_comment_lines(node, src),
        referenced_fields: Vec::new(), // TODO(parser): extract field refs for C/C++
        null_check_fields: Vec::new(), // TODO(parser): extract null checks for C/C++
        switch_dispatch_target: None,  // TODO(parser): extract switch dispatch target for C/C++
        optional_param_count: 0,       // TODO(parser): C has no optional params, keep 0
        called_functions: Vec::new(),  // TODO(parser): extract function calls for C/C++
        cognitive_complexity: body.map(cognitive_complexity_c).unwrap_or(0),
    })
}

fn find_func_name<'a>(declarator: Node<'a>, src: &'a [u8]) -> Option<&'a str> {
    // function_declarator -> declarator (identifier or qualified_identifier)
    if declarator.kind() == "identifier" {
        return Some(node_text(declarator, src));
    }
    declarator
        .child_by_field_name("declarator")
        .and_then(|d| find_func_name(d, src))
}

fn extract_params(declarator: Node, src: &[u8]) -> (usize, Vec<String>) {
    let params = match declarator.child_by_field_name("parameters") {
        Some(p) => p,
        None => return (0, vec![]),
    };
    let mut count = 0;
    let mut types = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            count += 1;
            let ty = child
                .child_by_field_name("type")
                .map(|t| node_text(t, src).to_string())
                .unwrap_or_else(|| "int".into());
            types.push(ty);
        }
    }
    (count, types)
}

fn extract_class(node: Node, src: &[u8]) -> Option<ClassInfo> {
    let name = node_text(node.child_by_field_name("name")?, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body");
    let field_count = body.map(count_fields).unwrap_or(0);
    let method_count = body.map(count_methods).unwrap_or(0);

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        method_count,
        is_exported: true,
        delegating_method_count: 0,
        field_count,
        field_names: Vec::new(),
        has_behavior: method_count > 0,
        is_interface: false,
        parent_name: None,
        override_count: 0,
        self_call_count: 0,
        has_listener_field: false,
        has_notify_method: false,
    })
}

fn count_fields(body: Node) -> usize {
    let mut count = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            count += 1;
        }
    }
    count
}

fn count_methods(body: Node) -> usize {
    let mut count = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "function_definition" || child.kind() == "declaration" {
            count += 1;
        }
    }
    count
}

fn extract_include(node: Node, src: &[u8]) -> Option<ImportInfo> {
    let path = node.child_by_field_name("path")?;
    let text = node_text(path, src)
        .trim_matches(|c| c == '"' || c == '<' || c == '>')
        .to_string();
    Some(ImportInfo {
        source: text,
        line: node.start_position().row + 1,
    })
}

fn count_complexity(node: Node) -> usize {
    let mut c = 1usize;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| match n.kind() {
        "if_statement"
        | "for_statement"
        | "while_statement"
        | "do_statement"
        | "case_statement"
        | "catch_clause"
        | "conditional_expression" => c += 1,
        "binary_expression" => {
            if let Some(op) = n.child_by_field_name("operator") {
                let kind = op.kind();
                if kind == "&&" || kind == "||" {
                    c += 1;
                }
            }
        }
        _ => {}
    });
    c
}

fn max_chain_depth(node: Node) -> usize {
    let mut max = 0;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "field_expression" {
            let d = chain_len(n);
            if d > max {
                max = d;
            }
        }
    });
    max
}

fn chain_len(node: Node) -> usize {
    let mut depth = 0;
    let mut current = node;
    while current.kind() == "field_expression" || current.kind() == "call_expression" {
        if current.kind() == "field_expression" {
            depth += 1;
        }
        match current.child(0) {
            Some(c) => current = c,
            None => break,
        }
    }
    depth
}

fn count_case_labels(node: Node) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "case_statement" {
            count += 1;
        }
    });
    count
}

fn cognitive_complexity_c(node: tree_sitter::Node) -> usize {
    let mut score = 0;
    cc_walk_c(node, 0, &mut score);
    score
}

fn cc_walk_c(node: tree_sitter::Node, nesting: usize, score: &mut usize) {
    match node.kind() {
        "if_statement" => {
            *score += 1 + nesting;
            cc_children_c(node, nesting + 1, score);
            return;
        }
        "for_statement" | "while_statement" | "do_statement" => {
            *score += 1 + nesting;
            cc_children_c(node, nesting + 1, score);
            return;
        }
        "switch_statement" => {
            *score += 1 + nesting;
            cc_children_c(node, nesting + 1, score);
            return;
        }
        "else_clause" => {
            *score += 1;
        }
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator")
                && (op.kind() == "&&" || op.kind() == "||")
            {
                *score += 1;
            }
        }
        "catch_clause" => {
            *score += 1 + nesting;
            cc_children_c(node, nesting + 1, score);
            return;
        }
        _ => {}
    }
    cc_children_c(node, nesting, score);
}

fn cc_children_c(node: tree_sitter::Node, nesting: usize, score: &mut usize) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        cc_walk_c(child, nesting, score);
    }
}

fn count_comment_lines(node: Node, src: &[u8]) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "comment" {
            count += node_text(n, src).lines().count();
        }
    });
    count
}

fn hash_ast(node: Node) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_node(node, &mut hasher);
    hasher.finish()
}

fn hash_node(node: Node, hasher: &mut DefaultHasher) {
    node.kind().hash(hasher);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        hash_node(child, hasher);
    }
}

fn node_text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

fn visit_all<F: FnMut(Node)>(node: Node, cursor: &mut tree_sitter::TreeCursor, f: &mut F) {
    f(node);
    if cursor.goto_first_child() {
        loop {
            let child_node = cursor.node();
            let mut child_cursor = child_node.walk();
            visit_all(child_node, &mut child_cursor, f);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}
