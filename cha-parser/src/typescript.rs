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

        let mut ctx = ParseContext::new(src);
        ctx.collect_nodes(root, false);

        Some(SourceModel {
            language: "typescript".into(),
            total_lines: file.line_count(),
            functions: ctx.col.functions,
            classes: ctx.col.classes,
            imports: ctx.col.imports,
            comments: collect_comments(root, src),
        })
    }
}

/// Accumulator for collected AST items.
struct Collector {
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    imports: Vec<ImportInfo>,
}

/// Bundles source bytes and collector to eliminate repeated parameter passing.
struct ParseContext<'a> {
    src: &'a [u8],
    col: Collector,
}

impl<'a> ParseContext<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self {
            src,
            col: Collector {
                functions: Vec::new(),
                classes: Vec::new(),
                imports: Vec::new(),
            },
        }
    }

    fn collect_nodes(&mut self, node: Node, exported: bool) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_single_node(child, exported);
        }
    }

    fn collect_single_node(&mut self, child: Node, exported: bool) {
        match child.kind() {
            "export_statement" => self.collect_nodes(child, true),
            "function_declaration" | "method_definition" => self.push_function(child, exported),
            "lexical_declaration" | "variable_declaration" => {
                extract_arrow_functions(child, self.src, exported, &mut self.col.functions);
                self.collect_nodes(child, exported);
            }
            "class_declaration" => self.push_class(child, exported),
            "import_statement" => self.push_import(child),
            _ => self.collect_nodes(child, false),
        }
    }

    fn push_function(&mut self, node: Node, exported: bool) {
        if let Some(mut f) = extract_function(node, self.src) {
            f.is_exported = exported;
            self.col.functions.push(f);
        }
    }

    fn push_class(&mut self, node: Node, exported: bool) {
        if let Some(mut c) = extract_class(node, self.src) {
            c.is_exported = exported;
            self.col.classes.push(c);
        }
    }

    fn push_import(&mut self, node: Node) {
        if let Some(i) = extract_import(node, self.src) {
            self.col.imports.push(i);
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
        comment_lines: count_comment_lines(node),
        referenced_fields: collect_this_fields(body, src),
        null_check_fields: collect_null_checks_ts(body, src),
        switch_dispatch_target: extract_switch_target_ts(body, src),
        optional_param_count: count_optional_params_ts(node, src),
        called_functions: collect_calls_ts(body, src),
        cognitive_complexity: body.map(cognitive_complexity_ts).unwrap_or(0),
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
        comment_lines: count_comment_lines(value),
        referenced_fields: collect_this_fields(body, src),
        null_check_fields: collect_null_checks_ts(body, src),
        switch_dispatch_target: extract_switch_target_ts(body, src),
        optional_param_count: count_optional_params_ts(value, src),
        called_functions: collect_calls_ts(Some(value), src),
        cognitive_complexity: cognitive_complexity_ts(value),
    })
}

fn extract_class(node: Node, src: &[u8]) -> Option<ClassInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body")?;
    let (methods, delegating, fields, has_behavior, cb_fields) = scan_class_body(body, src);
    let is_interface =
        node.kind() == "interface_declaration" || node.kind() == "abstract_class_declaration";
    let has_listener_field = !cb_fields.is_empty();
    let has_notify_method = has_iterate_and_call_ts(body, src, &cb_fields);

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        method_count: methods,
        line_count: end_line - start_line + 1,
        is_exported: false,
        delegating_method_count: delegating,
        field_count: fields.len(),
        field_names: fields,
        has_behavior,
        is_interface,
        parent_name: extract_parent_name(node, src),
        override_count: 0,
        self_call_count: 0,
        has_listener_field,
        has_notify_method,
    })
}

/// Scan class body and return (method_count, delegating_count, field_names, has_behavior, callback_fields).
fn scan_class_body(body: Node, src: &[u8]) -> (usize, usize, Vec<String>, bool, Vec<String>) {
    let mut methods = 0;
    let mut delegating = 0;
    let mut fields = Vec::new();
    let mut callback_fields = Vec::new();
    let mut has_behavior = false;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "method_definition" => {
                let (is_behavior, is_delegating) = classify_method(child, src);
                methods += 1;
                has_behavior |= is_behavior;
                delegating += usize::from(is_delegating);
            }
            "public_field_definition" | "property_definition" => {
                if let Some(n) = child.child_by_field_name("name") {
                    let name = node_text(n, src).to_string();
                    if is_callback_collection_type_ts(child, src) {
                        callback_fields.push(name.clone());
                    }
                    fields.push(name);
                }
            }
            _ => {}
        }
    }
    (methods, delegating, fields, has_behavior, callback_fields)
}

/// Classify a method: returns (is_behavior, is_delegating).
fn classify_method(node: Node, src: &[u8]) -> (bool, bool) {
    let mname = node
        .child_by_field_name("name")
        .map(|n| node_text(n, src))
        .unwrap_or("");
    let is_behavior = !is_accessor_name(mname) && mname != "constructor";
    let is_delegating = node
        .child_by_field_name("body")
        .is_some_and(|b| check_delegating(b, src));
    (is_behavior, is_delegating)
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

/// Count comment lines inside a node (recursive).
fn count_comment_lines(node: Node) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "comment" {
            count += child.end_position().row - child.start_position().row + 1;
        } else if child.child_count() > 0 {
            count += count_comment_lines(child);
        }
    }
    count
}

// cha:ignore todo_comment
/// Collect `this.xxx` field references from a function body.
fn collect_this_fields(body: Option<Node>, src: &[u8]) -> Vec<String> {
    let Some(body) = body else { return vec![] };
    let mut refs = Vec::new();
    collect_this_refs(body, src, &mut refs);
    refs.sort();
    refs.dedup();
    refs
}

fn collect_this_refs(node: Node, src: &[u8], refs: &mut Vec<String>) {
    if node.kind() == "member_expression"
        && let Some(obj) = node.child_by_field_name("object")
        && node_text(obj, src) == "this"
        && let Some(prop) = node.child_by_field_name("property")
    {
        refs.push(node_text(prop, src).to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_this_refs(child, src, refs);
    }
}

/// Check if a method name looks like a getter/setter.
fn is_accessor_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("get") || lower.starts_with("set") || lower.starts_with("is")
}

/// Extract parent class name from `extends` clause.
// cha:ignore cognitive_complexity
fn extract_parent_name(node: Node, src: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            let mut inner = child.walk();
            for c in child.children(&mut inner) {
                if c.kind() == "extends_clause" {
                    // First identifier child is the parent name
                    let mut ec = c.walk();
                    for e in c.children(&mut ec) {
                        if e.kind() == "identifier" || e.kind() == "type_identifier" {
                            return Some(node_text(e, src).to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Collect field names checked for null/undefined in TS.
fn collect_null_checks_ts(body: Option<Node>, src: &[u8]) -> Vec<String> {
    let Some(body) = body else { return vec![] };
    let mut fields = Vec::new();
    walk_null_checks_ts(body, src, &mut fields);
    fields.sort();
    fields.dedup();
    fields
}

fn walk_null_checks_ts(node: Node, src: &[u8], fields: &mut Vec<String>) {
    if node.kind() == "binary_expression"
        && let text = node_text(node, src)
        && (text.contains("null") || text.contains("undefined"))
        && let Some(left) = node.child_by_field_name("left")
        && let ltext = node_text(left, src)
        && let Some(f) = ltext.strip_prefix("this.")
    {
        fields.push(f.to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_null_checks_ts(child, src, fields);
    }
}

/// Extract switch dispatch target in TS.
fn extract_switch_target_ts(body: Option<Node>, src: &[u8]) -> Option<String> {
    let body = body?;
    find_switch_target_ts(body, src)
}

fn find_switch_target_ts(node: Node, src: &[u8]) -> Option<String> {
    if node.kind() == "switch_statement"
        && let Some(value) = node.child_by_field_name("value")
    {
        return Some(node_text(value, src).to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(t) = find_switch_target_ts(child, src) {
            return Some(t);
        }
    }
    None
}

/// Count optional parameters in TS (those with ? or default value).
fn count_optional_params_ts(node: Node, src: &[u8]) -> usize {
    let Some(params) = node.child_by_field_name("parameters") else {
        return 0;
    };
    let mut count = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        let text = node_text(child, src);
        if text.contains('?') || child.child_by_field_name("value").is_some() {
            count += 1;
        }
    }
    count
}

/// Check if a field's type annotation is a callback collection.
/// Matches: `Function[]`, `Array<Function>`, `(() => void)[]`, `((x: T) => R)[]`
fn is_callback_collection_type_ts(field_node: Node, src: &[u8]) -> bool {
    let Some(ty) = field_node.child_by_field_name("type") else {
        // No type annotation — check initializer for array literal
        if let Some(init) = field_node.child_by_field_name("value") {
            let text = node_text(init, src);
            return text == "[]" || text.contains("new Array");
        }
        return false;
    };
    let text = node_text(ty, src);
    // Function[] or Array<Function> or (() => void)[] or ((...) => ...)[]
    (text.contains("Function") && (text.contains("[]") || text.contains("Array<")))
        || (text.contains("=>") && text.contains("[]"))
        || text.contains("Array<(")
}

/// Structural Observer detection for TS: method iterates a callback field and calls elements.
/// Pattern: `this.field.forEach(cb => cb(...))` or `for (const cb of this.field) { cb(...) }`
fn has_iterate_and_call_ts(body: Node, src: &[u8], cb_fields: &[String]) -> bool {
    if cb_fields.is_empty() {
        return false;
    }
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "method_definition"
            && let Some(fn_body) = child.child_by_field_name("body")
        {
            for field in cb_fields {
                let this_field = format!("this.{field}");
                if walk_for_iterate_call_ts(fn_body, src, &this_field) {
                    return true;
                }
            }
        }
    }
    false
}

fn walk_for_iterate_call_ts(node: Node, src: &[u8], this_field: &str) -> bool {
    // for (const x of this.field) { x(...) }
    if node.kind() == "for_in_statement"
        && node_text(node, src).contains(this_field)
        && let Some(loop_body) = node.child_by_field_name("body")
        && has_call_expression_ts(loop_body)
    {
        return true;
    }
    // this.field.forEach(cb => cb(...))
    if node.kind() == "call_expression" || node.kind() == "expression_statement" {
        let text = node_text(node, src);
        if text.contains(this_field) && text.contains("forEach") {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if walk_for_iterate_call_ts(child, src, this_field) {
            return true;
        }
    }
    false
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

fn has_call_expression_ts(node: Node) -> bool {
    if node.kind() == "call_expression" {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_call_expression_ts(child) {
            return true;
        }
    }
    false
}

fn cognitive_complexity_ts(node: tree_sitter::Node) -> usize {
    let mut score = 0;
    cc_walk_ts(node, 0, &mut score);
    score
}

fn cc_walk_ts(node: tree_sitter::Node, nesting: usize, score: &mut usize) {
    match node.kind() {
        "if_statement" => {
            *score += 1 + nesting;
            cc_children_ts(node, nesting + 1, score);
            return;
        }
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement" => {
            *score += 1 + nesting;
            cc_children_ts(node, nesting + 1, score);
            return;
        }
        "switch_statement" => {
            *score += 1 + nesting;
            cc_children_ts(node, nesting + 1, score);
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
            cc_children_ts(node, nesting + 1, score);
            return;
        }
        "arrow_function" | "function_expression" => {
            cc_children_ts(node, nesting + 1, score);
            return;
        }
        _ => {}
    }
    cc_children_ts(node, nesting, score);
}

fn cc_children_ts(node: tree_sitter::Node, nesting: usize, score: &mut usize) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        cc_walk_ts(child, nesting, score);
    }
}

fn collect_calls_ts(body: Option<tree_sitter::Node>, src: &[u8]) -> Vec<String> {
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
