use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use cha_core::{ClassInfo, FunctionInfo, ImportInfo, SourceFile, SourceModel};
use tree_sitter::{Node, Parser};

use crate::LanguageParser;

pub struct PythonParser;

impl LanguageParser for PythonParser {
    fn language_name(&self) -> &str {
        "python"
    }

    fn parse(&self, file: &SourceFile) -> Option<SourceModel> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .ok()?;
        let tree = parser.parse(&file.content, None)?;
        let root = tree.root_node();
        let src = file.content.as_bytes();

        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        collect_top_level(root, src, &mut functions, &mut classes, &mut imports);

        Some(SourceModel {
            language: "python".into(),
            total_lines: file.line_count(),
            functions,
            classes,
            imports,
        })
    }
}

fn push_definition(
    node: Node,
    src: &[u8],
    functions: &mut Vec<FunctionInfo>,
    classes: &mut Vec<ClassInfo>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(f) = extract_function(node, src) {
                functions.push(f);
            }
        }
        "class_definition" => {
            if let Some(c) = extract_class(node, src, functions) {
                classes.push(c);
            }
        }
        _ => {}
    }
}

fn collect_top_level(
    node: Node,
    src: &[u8],
    functions: &mut Vec<FunctionInfo>,
    classes: &mut Vec<ClassInfo>,
    imports: &mut Vec<ImportInfo>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" | "class_definition" => {
                push_definition(child, src, functions, classes);
            }
            "import_statement" => collect_import(child, src, imports),
            "import_from_statement" => collect_import_from(child, src, imports),
            "decorated_definition" => {
                let mut inner = child.walk();
                for c in child.children(&mut inner) {
                    push_definition(c, src, functions, classes);
                }
            }
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

    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        complexity: count_complexity(node),
        body_hash: body.map(hash_ast_structure),
        is_exported: true,
        parameter_count: param_count,
        parameter_types: param_types,
        chain_depth: body.map(max_chain_depth).unwrap_or(0),
        switch_arms: body.map(count_match_arms).unwrap_or(0),
        external_refs: body
            .map(|b| collect_external_refs(b, src))
            .unwrap_or_default(),
        is_delegating: body.map(|b| check_delegating(b, src)).unwrap_or(false),
        comment_lines: count_comment_lines(node, src),
        referenced_fields: body.map(|b| collect_self_refs(b, src)).unwrap_or_default(),
        null_check_fields: body
            .map(|b| collect_none_checks(b, src))
            .unwrap_or_default(),
        switch_dispatch_target: None,
        optional_param_count: params.map(count_optional).unwrap_or(0),
        called_functions: body.map(|b| collect_calls_py(b, src)).unwrap_or_default(),
    })
}

fn find_method_def(child: Node) -> Option<Node> {
    if child.kind() == "function_definition" {
        return Some(child);
    }
    if child.kind() == "decorated_definition" {
        let mut inner = child.walk();
        return child
            .children(&mut inner)
            .find(|c| c.kind() == "function_definition");
    }
    None
}

fn extract_parent_name(node: Node, src: &[u8]) -> Option<String> {
    node.child_by_field_name("superclasses").and_then(|sc| {
        let mut c = sc.walk();
        sc.children(&mut c)
            .find(|n| n.kind() != "(" && n.kind() != ")" && n.kind() != ",")
            .map(|n| node_text(n, src).to_string())
    })
}

fn has_listener_name(name: &str) -> bool {
    name.contains("listener")
        || name.contains("handler")
        || name.contains("callback")
        || name.contains("observer")
}

fn process_method(
    func_node: Node,
    f: &mut FunctionInfo,
    src: &[u8],
    field_names: &mut Vec<String>,
) -> (bool, bool, bool, usize) {
    let method_name = &f.name;
    let mut has_behavior = false;
    let mut is_override = false;
    let mut is_notify = false;
    if method_name == "__init__" {
        collect_init_fields(func_node, src, field_names);
    } else {
        has_behavior = true;
    }
    let sc = func_node
        .child_by_field_name("body")
        .map(|b| count_self_calls(b, src))
        .unwrap_or(0);
    if method_name.starts_with("__") && method_name.ends_with("__") && method_name != "__init__" {
        is_override = true;
    }
    if method_name.contains("notify") || method_name.contains("emit") {
        is_notify = true;
    }
    f.is_exported = !method_name.starts_with('_');
    (has_behavior, is_override, is_notify, sc)
}

struct ClassScan {
    methods: Vec<FunctionInfo>,
    field_names: Vec<String>,
    delegating_count: usize,
    has_behavior: bool,
    override_count: usize,
    self_call_count: usize,
    has_notify_method: bool,
}

fn scan_class_methods(body: Node, src: &[u8]) -> ClassScan {
    let mut s = ClassScan {
        methods: Vec::new(),
        field_names: Vec::new(),
        delegating_count: 0,
        has_behavior: false,
        override_count: 0,
        self_call_count: 0,
        has_notify_method: false,
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        let Some(func_node) = find_method_def(child) else {
            continue;
        };
        let Some(mut f) = extract_function(func_node, src) else {
            continue;
        };
        if f.is_delegating {
            s.delegating_count += 1;
        }
        let (behav, over, notify, sc) = process_method(func_node, &mut f, src, &mut s.field_names);
        s.has_behavior |= behav;
        if over {
            s.override_count += 1;
        }
        if notify {
            s.has_notify_method = true;
        }
        s.self_call_count += sc;
        s.methods.push(f);
    }
    s
}

fn extract_class(
    node: Node,
    src: &[u8],
    top_functions: &mut Vec<FunctionInfo>,
) -> Option<ClassInfo> {
    let name = node_text(node.child_by_field_name("name")?, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body")?;
    let s = scan_class_methods(body, src);
    let method_count = s.methods.len();
    top_functions.extend(s.methods);

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        line_count: end_line - start_line + 1,
        method_count,
        is_exported: true,
        delegating_method_count: s.delegating_count,
        field_count: s.field_names.len(),
        has_listener_field: s.field_names.iter().any(|n| has_listener_name(n)),
        field_names: s.field_names,
        has_behavior: s.has_behavior,
        is_interface: has_only_pass_or_ellipsis(body, src),
        parent_name: extract_parent_name(node, src),
        override_count: s.override_count,
        self_call_count: s.self_call_count,
        has_notify_method: s.has_notify_method,
    })
}

// --- imports ---

fn collect_import(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let line = node.start_position().row + 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
            let text = node_text(child, src);
            imports.push(ImportInfo {
                source: text.to_string(),
                line,
            });
        }
    }
}

fn collect_import_from(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let line = node.start_position().row + 1;
    let module = node
        .child_by_field_name("module_name")
        .map(|n| node_text(n, src).to_string())
        .unwrap_or_default();
    let mut cursor = node.walk();
    let mut has_names = false;
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
            let n = node_text(child, src).to_string();
            if n != module {
                imports.push(ImportInfo {
                    source: format!("{module}.{n}"),
                    line,
                });
                has_names = true;
            }
        }
    }
    if !has_names {
        imports.push(ImportInfo {
            source: module,
            line,
        });
    }
}

// --- helpers ---

fn node_text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

fn count_complexity(node: Node) -> usize {
    let mut complexity = 1usize;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        match n.kind() {
            "if_statement"
            | "elif_clause"
            | "for_statement"
            | "while_statement"
            | "except_clause"
            | "with_statement"
            | "assert_statement"
            | "conditional_expression"
            | "boolean_operator"
            | "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression" => {
                complexity += 1;
            }
            "match_statement" => {} // match itself doesn't add, cases do
            "case_clause" => {
                complexity += 1;
            }
            _ => {}
        }
    });
    complexity
}

fn hash_ast_structure(node: Node) -> u64 {
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

fn max_chain_depth(node: Node) -> usize {
    let mut max = 0usize;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "attribute" {
            let depth = chain_len(n);
            if depth > max {
                max = depth;
            }
        }
    });
    max
}

fn chain_len(node: Node) -> usize {
    let mut depth = 0usize;
    let mut current = node;
    while current.kind() == "attribute" || current.kind() == "call" {
        if current.kind() == "attribute" {
            depth += 1;
        }
        if let Some(obj) = current.child(0) {
            current = obj;
        } else {
            break;
        }
    }
    depth
}

fn count_match_arms(node: Node) -> usize {
    let mut count = 0usize;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "case_clause" {
            count += 1;
        }
    });
    count
}

fn collect_external_refs(node: Node, src: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() != "attribute" {
            return;
        }
        let Some(obj) = n.child(0) else { return };
        let text = node_text(obj, src);
        if text != "self"
            && !text.is_empty()
            && text.starts_with(|c: char| c.is_lowercase())
            && !refs.contains(&text.to_string())
        {
            refs.push(text.to_string());
        }
    });
    refs
}

fn unwrap_single_call(body: Node) -> Option<Node> {
    let mut c = body.walk();
    let stmts: Vec<Node> = body
        .children(&mut c)
        .filter(|n| !n.is_extra() && n.kind() != "pass_statement" && n.kind() != "comment")
        .collect();
    if stmts.len() != 1 {
        return None;
    }
    let stmt = stmts[0];
    match stmt.kind() {
        "return_statement" => stmt.child(1).filter(|v| v.kind() == "call"),
        "expression_statement" => stmt.child(0).filter(|v| v.kind() == "call"),
        _ => None,
    }
}

fn check_delegating(body: Node, src: &[u8]) -> bool {
    let Some(func) = unwrap_single_call(body).and_then(|c| c.child(0)) else {
        return false;
    };
    let text = node_text(func, src);
    text.contains('.') && !text.starts_with("self.")
}

fn count_comment_lines(node: Node, src: &[u8]) -> usize {
    let mut count = 0usize;
    let mut cursor = node.walk();
    visit_all(node, &mut cursor, &mut |n| {
        if n.kind() == "comment" {
            count += 1;
        } else if n.kind() == "string" || n.kind() == "expression_statement" {
            // docstrings
            let text = node_text(n, src);
            if text.starts_with("\"\"\"") || text.starts_with("'''") {
                count += text.lines().count() as usize;
            }
        }
    });
    count
}

fn collect_self_refs(body: Node, src: &[u8]) -> Vec<String> {
    let mut refs = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() != "attribute" {
            return;
        }
        let is_self = n.child(0).is_some_and(|o| node_text(o, src) == "self");
        if !is_self {
            return;
        }
        if let Some(attr) = n.child_by_field_name("attribute") {
            let name = node_text(attr, src).to_string();
            if !refs.contains(&name) {
                refs.push(name);
            }
        }
    });
    refs
}

fn collect_none_checks(body: Node, src: &[u8]) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() != "comparison_operator" {
            return;
        }
        let text = node_text(n, src);
        if !text.contains("is None") && !text.contains("is not None") && !text.contains("== None") {
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

fn is_self_or_cls(name: &str) -> bool {
    name == "self" || name == "cls"
}

fn param_name_and_type(child: Node, src: &[u8]) -> Option<(String, String)> {
    match child.kind() {
        "identifier" => {
            let name = node_text(child, src);
            (!is_self_or_cls(name)).then(|| (name.to_string(), "Any".to_string()))
        }
        "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
            let name = child
                .child_by_field_name("name")
                .or_else(|| child.child(0))
                .map(|n| node_text(n, src))
                .unwrap_or("");
            if is_self_or_cls(name) {
                return None;
            }
            let ty = child
                .child_by_field_name("type")
                .map(|n| node_text(n, src).to_string())
                .unwrap_or_else(|| "Any".to_string());
            Some((name.to_string(), ty))
        }
        "list_splat_pattern" | "dictionary_splat_pattern" => {
            Some(("*".to_string(), "Any".to_string()))
        }
        _ => None,
    }
}

fn extract_params(params_node: Node, src: &[u8]) -> (usize, Vec<String>) {
    let mut count = 0usize;
    let mut types = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if let Some((_name, ty)) = param_name_and_type(child, src) {
            count += 1;
            types.push(ty);
        }
    }
    (count, types)
}

fn count_optional(params_node: Node) -> usize {
    let mut count = 0usize;
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if child.kind() == "default_parameter" || child.kind() == "typed_default_parameter" {
            count += 1;
        }
    }
    count
}

fn collect_init_fields(func_node: Node, src: &[u8], fields: &mut Vec<String>) {
    let Some(body) = func_node.child_by_field_name("body") else {
        return;
    };
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() != "assignment" {
            return;
        }
        let Some(left) = n.child_by_field_name("left") else {
            return;
        };
        if left.kind() != "attribute" {
            return;
        }
        let is_self = left.child(0).is_some_and(|o| node_text(o, src) == "self");
        if !is_self {
            return;
        }
        if let Some(attr) = left.child_by_field_name("attribute") {
            let name = node_text(attr, src).to_string();
            if !fields.contains(&name) {
                fields.push(name);
            }
        }
    });
}

fn count_self_calls(body: Node, src: &[u8]) -> usize {
    let mut count = 0;
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() != "call" {
            return;
        }
        let is_self_call = n
            .child(0)
            .filter(|f| f.kind() == "attribute")
            .and_then(|f| f.child(0))
            .is_some_and(|obj| node_text(obj, src) == "self");
        if is_self_call {
            count += 1;
        }
    });
    count
}

fn is_stub_body(node: Node, src: &[u8]) -> bool {
    node.child_by_field_name("body")
        .is_none_or(|b| has_only_pass_or_ellipsis(b, src))
}

fn has_only_pass_or_ellipsis(body: Node, src: &[u8]) -> bool {
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        let ok = match child.kind() {
            "pass_statement" | "ellipsis" | "comment" => true,
            "expression_statement" => child.child(0).is_none_or(|expr| {
                let text = node_text(expr, src);
                text == "..." || text.starts_with("\"\"\"") || text.starts_with("'''")
            }),
            "function_definition" => is_stub_body(child, src),
            "decorated_definition" => {
                let mut inner = child.walk();
                child
                    .children(&mut inner)
                    .filter(|c| c.kind() == "function_definition")
                    .all(|c| is_stub_body(c, src))
            }
            _ => false,
        };
        if !ok {
            return false;
        }
    }
    true
}

fn collect_calls_py(body: tree_sitter::Node, src: &[u8]) -> Vec<String> {
    let mut calls = Vec::new();
    let mut cursor = body.walk();
    visit_all(body, &mut cursor, &mut |n| {
        if n.kind() == "call"
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
