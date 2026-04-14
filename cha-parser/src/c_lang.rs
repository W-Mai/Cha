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
        comments: collect_comments(root, src),
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
            "type_definition" => extract_typedef_struct(child, src, classes),
            "preproc_include" => {
                if let Some(imp) = extract_include(child, src) {
                    imports.push(imp);
                }
            }
            _ => {
                if child.child_count() > 0 {
                    collect_top_level(child, src, functions, classes, imports);
                }
            }
        }
    }
}

fn extract_typedef_struct(node: Node, src: &[u8], classes: &mut Vec<ClassInfo>) {
    let mut inner = node.walk();
    for sub in node.children(&mut inner) {
        if sub.kind() != "struct_specifier" && sub.kind() != "class_specifier" {
            continue;
        }
        let Some(mut c) = extract_class(sub, src) else {
            continue;
        };
        if c.name.is_empty()
            && let Some(decl) = node.child_by_field_name("declarator")
        {
            c.name = node_text(decl, src).to_string();
        }
        if !c.name.is_empty() {
            classes.push(c);
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
        external_refs: body
            .map(|b| collect_external_refs_c(b, src))
            .unwrap_or_default(),
        is_delegating: body.map(|b| check_delegating_c(b, src)).unwrap_or(false),
        comment_lines: count_comment_lines(node, src),
        referenced_fields: body
            .map(|b| collect_field_refs_c(b, src))
            .unwrap_or_default(),
        null_check_fields: body
            .map(|b| collect_null_checks_c(b, src))
            .unwrap_or_default(),
        switch_dispatch_target: body.and_then(|b| extract_switch_target_c(b, src)),
        optional_param_count: 0,
        called_functions: body.map(|b| collect_calls_c(b, src)).unwrap_or_default(),
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
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(n, src).to_string())
        .unwrap_or_default();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body");
    let method_count = body.map(count_methods).unwrap_or(0);
    let (field_names, field_types, first_field_type) =
        body.map(|b| extract_field_info(b, src)).unwrap_or_default();

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        method_count,
        is_exported: true,
        delegating_method_count: 0,
        field_count: field_names.len(),
        field_names,
        field_types,
        has_behavior: method_count > 0,
        is_interface: false,
        // First field type stored as parent candidate;
        // build_class_graph validates against known struct names.
        parent_name: first_field_type,
        override_count: 0,
        self_call_count: 0,
        has_listener_field: false,
        has_notify_method: false,
    })
}

fn extract_field_info(body: Node, src: &[u8]) -> (Vec<String>, Vec<String>, Option<String>) {
    let mut names = Vec::new();
    let mut types = Vec::new();
    let mut first_type = None;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            if let Some(decl) = child.child_by_field_name("declarator") {
                names.push(node_text(decl, src).to_string());
            }
            let ty = child
                .child_by_field_name("type")
                .map(|t| node_text(t, src).to_string());
            if first_type.is_none() {
                first_type = ty.clone();
            }
            types.push(ty.unwrap_or_default());
        }
    }
    (names, types, first_type)
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

fn collect_external_refs_c(body: Node, src: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() == "field_expression"
            && let Some(obj) = n.child(0)
            && obj.kind() == "identifier"
        {
            let name = node_text(obj, src).to_string();
            if !refs.contains(&name) {
                refs.push(name);
            }
        }
    });
    refs
}

fn check_delegating_c(body: Node, src: &[u8]) -> bool {
    let mut cursor = body.walk();
    let stmts: Vec<Node> = body
        .children(&mut cursor)
        .filter(|n| n.kind() != "{" && n.kind() != "}" && !n.kind().contains("comment"))
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
        .is_some_and(|f| node_text(f, src).contains('.') || node_text(f, src).contains("->"))
}

fn collect_field_refs_c(body: Node, src: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() == "field_expression"
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

fn collect_null_checks_c(body: Node, src: &[u8]) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() == "binary_expression" {
            let text = node_text(n, src);
            if (text.contains("NULL") || text.contains("nullptr"))
                && let Some(left) = n.child(0)
            {
                let name = node_text(left, src).to_string();
                if !fields.contains(&name) {
                    fields.push(name);
                }
            }
        }
    });
    fields
}

fn extract_switch_target_c(body: Node, src: &[u8]) -> Option<String> {
    let mut cursor = body.walk();
    let mut target = None;
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() == "switch_statement"
            && target.is_none()
            && let Some(cond) = n.child_by_field_name("condition")
        {
            target = Some(node_text(cond, src).trim_matches(['(', ')']).to_string());
        }
    });
    target
}

fn collect_calls_c(body: Node, src: &[u8]) -> Vec<String> {
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
