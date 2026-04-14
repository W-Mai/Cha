use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use cha_core::{ClassInfo, FunctionInfo, ImportInfo, SourceFile, SourceModel};
use tree_sitter::{Node, Parser};

use crate::LanguageParser;

pub struct GolangParser;

impl LanguageParser for GolangParser {
    fn language_name(&self) -> &str {
        "go"
    }

    fn parse(&self, file: &SourceFile) -> Option<SourceModel> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into()).ok()?;
        let tree = parser.parse(&file.content, None)?;
        let root = tree.root_node();
        let src = file.content.as_bytes();

        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        collect_top_level(root, src, &mut functions, &mut classes, &mut imports);

        Some(SourceModel {
            language: "go".into(),
            total_lines: file.line_count(),
            functions,
            classes,
            imports,
        })
    }
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
            "function_declaration" | "method_declaration" => {
                if let Some(f) = extract_function(child, src) {
                    functions.push(f);
                }
            }
            "type_declaration" => extract_type_decl(child, src, classes),
            "import_declaration" => collect_imports(child, src, imports),
            _ => {}
        }
    }
}

fn extract_function(node: Node, src: &[u8]) -> Option<FunctionInfo> {
    let name = node_text(node.child_by_field_name("name")?, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body");
    let params = node.child_by_field_name("parameters");
    let (param_count, param_types) = params
        .map(|p| extract_params(p, src))
        .unwrap_or((0, vec![]));
    let is_exported = name.starts_with(|c: char| c.is_uppercase());

    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        complexity: count_complexity(node),
        body_hash: body.map(hash_ast),
        is_exported,
        parameter_count: param_count,
        parameter_types: param_types,
        chain_depth: body.map(max_chain_depth).unwrap_or(0),
        switch_arms: body.map(count_case_clauses).unwrap_or(0),
        external_refs: body
            .map(|b| collect_external_refs(b, src))
            .unwrap_or_default(),
        is_delegating: body.map(|b| check_delegating(b, src)).unwrap_or(false),
        comment_lines: count_comment_lines(node, src),
        referenced_fields: Vec::new(), // TODO(parser): extract field refs for Go
        null_check_fields: body.map(|b| collect_nil_checks(b, src)).unwrap_or_default(),
        switch_dispatch_target: None, // TODO(parser): extract switch dispatch target for Go
        optional_param_count: 0,      // TODO(parser): Go has no optional params, keep 0
        called_functions: collect_calls(body, src),
        cognitive_complexity: body.map(|b| cognitive_complexity_go(b)).unwrap_or(0),
    })
}

fn extract_type_decl(node: Node, src: &[u8], classes: &mut Vec<ClassInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_spec"
            && let Some(c) = extract_struct(child, src)
        {
            classes.push(c);
        }
    }
}

fn extract_struct(node: Node, src: &[u8]) -> Option<ClassInfo> {
    let name = node_text(node.child_by_field_name("name")?, src).to_string();
    let type_node = node.child_by_field_name("type")?;
    if type_node.kind() != "struct_type" && type_node.kind() != "interface_type" {
        return None;
    }
    let is_interface = type_node.kind() == "interface_type";
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let field_count = count_struct_fields(type_node);
    let is_exported = name.starts_with(|c: char| c.is_uppercase());

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        method_count: 0,
        is_exported,
        delegating_method_count: 0,
        field_count,
        field_names: Vec::new(),
        has_behavior: false,
        is_interface,
        parent_name: None,
        override_count: 0,
        self_call_count: 0,
        has_listener_field: false,
        has_notify_method: false,
    })
}

fn count_struct_fields(node: Node) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "field_declaration" {
            count += 1;
        }
    });
    count
}

fn collect_imports(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "import_spec" {
            let line = n.start_position().row + 1;
            let path_node = n.child_by_field_name("path").unwrap_or(n);
            let text = node_text(path_node, src).trim_matches('"').to_string();
            if !text.is_empty() {
                imports.push(ImportInfo { source: text, line });
            }
        }
    });
}

fn extract_params(params: Node, src: &[u8]) -> (usize, Vec<String>) {
    let mut count = 0;
    let mut types = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            let ty = child
                .child_by_field_name("type")
                .map(|t| node_text(t, src).to_string())
                .unwrap_or_else(|| "any".into());
            // Count names in this declaration (e.g. `a, b int` = 2 params)
            let mut inner = child.walk();
            let names: usize = child
                .children(&mut inner)
                .filter(|c| c.kind() == "identifier")
                .count()
                .max(1);
            for _ in 0..names {
                count += 1;
                types.push(ty.clone());
            }
        }
    }
    (count, types)
}

fn count_complexity(node: Node) -> usize {
    let mut c = 1usize;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| match n.kind() {
        "if_statement" | "for_statement" | "expression_case" | "default_case" | "type_case"
        | "select_statement" | "go_statement" => c += 1,
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
        if n.kind() == "selector_expression" {
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
    while current.kind() == "selector_expression" || current.kind() == "call_expression" {
        if current.kind() == "selector_expression" {
            depth += 1;
        }
        match current.child(0) {
            Some(c) => current = c,
            None => break,
        }
    }
    depth
}

fn count_case_clauses(node: Node) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "expression_case" || n.kind() == "default_case" || n.kind() == "type_case" {
            count += 1;
        }
    });
    count
}

fn collect_external_refs(node: Node, src: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "selector_expression"
            && let Some(obj) = n.child(0)
            && obj.kind() == "identifier"
        {
            let text = node_text(obj, src).to_string();
            if !refs.contains(&text) {
                refs.push(text);
            }
        }
    });
    refs
}

fn check_delegating(body: Node, src: &[u8]) -> bool {
    let mut cursor = body.walk();
    let stmts: Vec<Node> = body
        .children(&mut cursor)
        .filter(|n| n.kind() != "{" && n.kind() != "}" && n.kind() != "comment")
        .collect();
    if stmts.len() != 1 {
        return false;
    }
    let stmt = stmts[0];
    let call = match stmt.kind() {
        "return_statement" => stmt.child(1).filter(|c| c.kind() == "call_expression"),
        "expression_statement" => stmt.child(0).filter(|c| c.kind() == "call_expression"),
        _ => None,
    };
    call.and_then(|c| c.child(0))
        .is_some_and(|f| node_text(f, src).contains('.'))
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

fn collect_nil_checks(body: Node, src: &[u8]) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() != "binary_expression" {
            return;
        }
        let text = node_text(n, src);
        if !text.contains("nil") {
            return;
        }
        if let Some(left) = n.child(0) {
            let name = node_text(left, src).to_string();
            if !fields.contains(&name) {
                fields.push(name);
            }
        }
    });
    fields
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

fn cognitive_complexity_go(node: Node) -> usize {
    let mut score = 0;
    cc_walk(node, 0, &mut score);
    score
}

fn cc_walk(node: Node, nesting: usize, score: &mut usize) {
    match node.kind() {
        "if_statement" => {
            *score += 1 + nesting;
            cc_children(node, nesting + 1, score);
            return;
        }
        "for_statement" => {
            *score += 1 + nesting;
            cc_children(node, nesting + 1, score);
            return;
        }
        "expression_switch_statement" | "type_switch_statement" | "select_statement" => {
            *score += 1 + nesting;
            cc_children(node, nesting + 1, score);
            return;
        }
        "else_clause" => {
            *score += 1; // no nesting increment for else
        }
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator")
                && (op.kind() == "&&" || op.kind() == "||")
            {
                *score += 1;
            }
        }
        _ => {}
    }
    cc_children(node, nesting, score);
}

fn cc_children(node: Node, nesting: usize, score: &mut usize) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        cc_walk(child, nesting, score);
    }
}

fn collect_calls(body: Option<Node>, src: &[u8]) -> Vec<String> {
    let Some(body) = body else { return Vec::new() };
    let mut calls = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() == "call_expression"
            && let Some(func) = n.child(0)
        {
            let name = node_text(func, src).to_string();
            if !calls.contains(&name) {
                calls.push(name);
            }
        }
    });
    calls
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
