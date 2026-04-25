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

        let imports_map = crate::golang_imports::build(root, src, &file.path);
        collect_top_level(
            root,
            src,
            &imports_map,
            &mut functions,
            &mut classes,
            &mut imports,
        );

        Some(SourceModel {
            language: "go".into(),
            total_lines: file.line_count(),
            functions,
            classes,
            imports,
            comments: collect_comments(root, src),
            type_aliases: vec![], // TODO(parser): extract type aliases from 'type X = Y' declarations
        })
    }
}

fn collect_top_level(
    root: Node,
    src: &[u8],
    imports_map: &crate::type_ref::ImportsMap,
    functions: &mut Vec<FunctionInfo>,
    classes: &mut Vec<ClassInfo>,
    imports: &mut Vec<ImportInfo>,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "method_declaration" => {
                if let Some(f) = extract_function(child, src, imports_map) {
                    functions.push(f);
                }
            }
            "type_declaration" => extract_type_decl(child, src, classes),
            "import_declaration" => collect_imports(child, src, imports),
            _ => {}
        }
    }
}

fn extract_function(
    node: Node,
    src: &[u8],
    imports_map: &crate::type_ref::ImportsMap,
) -> Option<FunctionInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let name_col = name_node.start_position().column;
    let name_end_col = name_node.end_position().column;
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body");
    let params = node.child_by_field_name("parameters");
    let (param_count, param_types) = params
        .map(|p| extract_params(p, src, imports_map))
        .unwrap_or((0, vec![]));
    let is_exported = name.starts_with(|c: char| c.is_uppercase());

    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        name_col,
        name_end_col,
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
        referenced_fields: body
            .map(|b| collect_field_refs_go(b, src))
            .unwrap_or_default(),
        null_check_fields: body.map(|b| collect_nil_checks(b, src)).unwrap_or_default(),
        switch_dispatch_target: body.and_then(|b| extract_switch_target_go(b, src)),
        optional_param_count: 0,
        called_functions: collect_calls(body, src),
        cognitive_complexity: body.map(|b| cognitive_complexity_go(b)).unwrap_or(0),
        return_type: node
            .child_by_field_name("result")
            .map(|rt| crate::type_ref::resolve(node_text(rt, src), imports_map)),
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
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let name_col = name_node.start_position().column;
    let name_end_col = name_node.end_position().column;
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
        name_col,
        name_end_col,
        line_count: end_line - start_line + 1,
        method_count: 0,
        is_exported,
        delegating_method_count: 0,
        field_count,
        field_names: Vec::new(),
        field_types: Vec::new(),
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
            let col = n.start_position().column;
            let path_node = n.child_by_field_name("path").unwrap_or(n);
            let text = node_text(path_node, src).trim_matches('"').to_string();
            if !text.is_empty() {
                imports.push(ImportInfo {
                    source: text,
                    line,
                    col,
                    ..Default::default()
                });
            }
        }
    });
}

fn extract_params(
    params: Node,
    src: &[u8],
    imports_map: &crate::type_ref::ImportsMap,
) -> (usize, Vec<cha_core::TypeRef>) {
    let mut count = 0;
    let mut types = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            let raw = child
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
                types.push(resolve_go_type(&raw, imports_map));
            }
        }
    }
    (count, types)
}

/// Go types are `pkg.TypeName` or `*pkg.TypeName`; the importable name in
/// ImportsMap is the package alias. Split on `.`, look up the first segment.
fn resolve_go_type(raw: &str, imports_map: &crate::type_ref::ImportsMap) -> cha_core::TypeRef {
    // Strip decorations to get the inner `pkg.Type` or `Type`.
    let inner = raw.trim_start_matches('*').trim_start_matches('[').trim();
    let inner = inner.trim_start_matches(']').trim();
    let mut parts = inner.splitn(2, '.');
    let first = parts.next().unwrap_or(inner);
    let second = parts.next();
    let (short_name, origin) = if let Some(type_part) = second {
        let origin = imports_map
            .get(first)
            .cloned()
            .unwrap_or(cha_core::TypeOrigin::Unknown);
        (type_part.to_string(), origin)
    } else {
        // No `.` → builtin type (string, int, bool, etc.) or locally-declared
        // type. Treat builtin primitives accordingly; everything else → Local.
        let origin = if is_go_builtin(inner) {
            cha_core::TypeOrigin::Primitive
        } else {
            cha_core::TypeOrigin::Local
        };
        (inner.to_string(), origin)
    };
    cha_core::TypeRef {
        name: short_name,
        raw: raw.to_string(),
        origin,
    }
}

fn is_go_builtin(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "byte"
            | "complex64"
            | "complex128"
            | "error"
            | "float32"
            | "float64"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "rune"
            | "string"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uintptr"
            | "any"
            | "interface{}"
    )
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

fn collect_field_refs_go(body: Node, src: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() == "selector_expression"
            && let Some(field) = n.child_by_field_name("field")
        {
            let name = node_text(field, src).to_string();
            if !refs.contains(&name) {
                refs.push(name);
            }
        }
    });
    refs
}

fn extract_switch_target_go(body: Node, src: &[u8]) -> Option<String> {
    let mut target = None;
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if (n.kind() == "expression_switch_statement" || n.kind() == "type_switch_statement")
            && target.is_none()
            && let Some(val) = n.child_by_field_name("value")
        {
            target = Some(node_text(val, src).to_string());
        }
    });
    target
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

fn collect_comments(root: Node, src: &[u8]) -> Vec<cha_core::CommentInfo> {
    let mut comments = Vec::new();
    let mut cursor = root.walk();
    visit_all(root, &mut cursor, &mut |n| {
        if n.kind().contains("comment") {
            comments.push(cha_core::CommentInfo {
                text: node_text(n, src).to_string(),
                line: n.start_position().row + 1,
            });
        }
    });
    comments
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
