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

        let mut ctx = ParseContext::new(src);
        ctx.collect_nodes(root, false);

        Some(SourceModel {
            language: "rust".into(),
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
    last_self_call_count: usize,
    last_has_notify: bool,
    /// Callback collection field names per class name.
    callback_fields: std::collections::HashMap<String, Vec<String>>,
}

impl<'a> ParseContext<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self {
            src,
            last_self_call_count: 0,
            last_has_notify: false,
            callback_fields: std::collections::HashMap::new(),
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
            "function_item" => self.push_function(child, exported),
            "impl_item" => self.extract_impl_methods(child),
            "struct_item" | "enum_item" | "trait_item" => self.push_struct(child),
            "use_declaration" => self.push_import(child),
            _ => self.collect_nodes(child, false),
        }
    }

    fn push_function(&mut self, node: Node, exported: bool) {
        if let Some(mut f) = extract_function(node, self.src) {
            f.is_exported = exported || has_pub(node);
            self.col.functions.push(f);
        }
    }

    fn push_struct(&mut self, node: Node) {
        if let Some((mut c, cb_fields)) = extract_struct(node, self.src) {
            c.is_exported = has_pub(node);
            if !cb_fields.is_empty() {
                self.callback_fields.insert(c.name.clone(), cb_fields);
            }
            self.col.classes.push(c);
        }
    }

    fn push_import(&mut self, node: Node) {
        if let Some(i) = extract_use(node, self.src) {
            self.col.imports.push(i);
        }
    }

    fn extract_impl_methods(&mut self, node: Node) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let impl_name = node
            .child_by_field_name("type")
            .map(|t| node_text(t, self.src).to_string());
        let trait_name = node
            .child_by_field_name("trait")
            .map(|t| node_text(t, self.src).to_string());

        let cb_fields = impl_name
            .as_ref()
            .and_then(|n| self.callback_fields.get(n))
            .cloned()
            .unwrap_or_default();

        let (methods, delegating, has_behavior) = self.scan_impl_body(body, &cb_fields);

        if let Some(name) = &impl_name
            && let Some(class) = self.col.classes.iter_mut().find(|c| &c.name == name)
        {
            class.method_count += methods;
            class.delegating_method_count += delegating;
            class.has_behavior |= has_behavior;
            class.self_call_count = class.self_call_count.max(self.last_self_call_count);
            class.has_notify_method |= self.last_has_notify;
            if let Some(t) = &trait_name {
                class.parent_name = Some(t.clone());
            }
        }
    }

    fn scan_impl_body(&mut self, body: Node, cb_fields: &[String]) -> (usize, usize, bool) {
        let mut methods = 0;
        let mut delegating = 0;
        let mut has_behavior = false;
        let mut max_self_calls = 0;
        let mut has_notify = false;
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item"
                && let Some(mut f) = extract_function(child, self.src)
            {
                f.is_exported = has_pub(child);
                methods += 1;
                if f.is_delegating {
                    delegating += 1;
                }
                if f.line_count > 3 {
                    has_behavior = true;
                }
                let fn_body = child.child_by_field_name("body");
                let self_calls = count_self_method_calls(fn_body, self.src);
                max_self_calls = max_self_calls.max(self_calls);
                // Structural Observer: method iterates a callback field and calls elements
                if !has_notify && has_iterate_and_call(fn_body, self.src, cb_fields) {
                    has_notify = true;
                }
                self.col.functions.push(f);
            }
        }
        // Store extra signals in the class via the caller
        self.last_self_call_count = max_self_calls;
        self.last_has_notify = has_notify;
        (methods, delegating, has_behavior)
    }
}

// Extract and push a function item node.
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
        comment_lines: count_comment_lines(node, src),
        referenced_fields: collect_field_refs(body, src),
        null_check_fields: collect_null_checks(body, src),
        switch_dispatch_target: extract_switch_target(body, src),
        optional_param_count: count_optional_params(node, src),
        called_functions: collect_calls_rs(body, src),
        cognitive_complexity: body.map(cognitive_complexity_rs).unwrap_or(0),
    })
}

fn extract_struct(node: Node, src: &[u8]) -> Option<(ClassInfo, Vec<String>)> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, src).to_string();
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let (field_count, field_names, callback_fields) = extract_fields(node, src);
    let is_interface = node.kind() == "trait_item";
    let has_listener_field = !callback_fields.is_empty();
    Some((
        ClassInfo {
            name,
            start_line,
            end_line,
            method_count: 0,
            line_count: end_line - start_line + 1,
            is_exported: false,
            delegating_method_count: 0,
            field_count,
            field_names,
            has_behavior: false,
            is_interface,
            parent_name: None,
            override_count: 0,
            self_call_count: 0,
            has_listener_field,
            has_notify_method: false,
        },
        callback_fields,
    ))
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
            types.push(normalize_type(node_text(ty, src)));
        }
    }
    types.sort();
    types
}

/// Strip reference/lifetime wrappers to get the canonical type name.
fn normalize_type(raw: &str) -> String {
    raw.trim_start_matches('&')
        .trim_start_matches("mut ")
        .trim()
        .to_string()
}

fn max_chain_depth(node: Node) -> usize {
    let mut max = 0;
    walk_chain_depth(node, &mut max);
    max
}

fn walk_chain_depth(node: Node, max: &mut usize) {
    if node.kind() == "field_expression" {
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

/// Count consecutive field accesses (a.b.c.d), skipping method calls.
fn measure_chain(node: Node) -> usize {
    let mut depth = 0;
    let mut current = node;
    while current.kind() == "field_expression" {
        depth += 1;
        if let Some(obj) = current.child_by_field_name("value") {
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

/// Walk a field_expression chain to its root identifier.
fn field_chain_root(node: Node) -> Node {
    let mut current = node;
    while current.kind() == "field_expression" {
        match current.child_by_field_name("value") {
            Some(child) => current = child,
            None => break,
        }
    }
    current
}

fn walk_external_refs(node: Node, src: &[u8], refs: &mut Vec<String>) {
    if node.kind() == "field_expression" {
        // Walk to the root object of the chain (a.b.c → a)
        let root = field_chain_root(node);
        let text = node_text(root, src);
        if text != "self" && !text.is_empty() {
            refs.push(text.to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_external_refs(child, src, refs);
    }
}

/// Extract the single statement from a block body, if exactly one exists.
fn single_stmt(body: Node) -> Option<Node> {
    let mut cursor = body.walk();
    let stmts: Vec<_> = body
        .children(&mut cursor)
        .filter(|c| c.kind() != "{" && c.kind() != "}")
        .collect();
    (stmts.len() == 1).then(|| stmts[0])
}

/// Check if a node is a method call on an external object (not self).
fn is_external_call(node: Node, src: &[u8]) -> bool {
    node.kind() == "call_expression"
        && node.child_by_field_name("function").is_some_and(|func| {
            func.kind() == "field_expression"
                && func
                    .child_by_field_name("value")
                    .is_some_and(|obj| node_text(obj, src) != "self")
        })
}

fn check_delegating(body: Node, src: &[u8]) -> bool {
    let Some(stmt) = single_stmt(body) else {
        return false;
    };
    let expr = match stmt.kind() {
        "expression_statement" => stmt.child(0).unwrap_or(stmt),
        "return_expression" => stmt.child(1).unwrap_or(stmt),
        _ => stmt,
    };
    is_external_call(expr, src)
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

/// Count comment lines inside a function node.
fn count_comment_lines(node: Node, src: &[u8]) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "line_comment" || child.kind() == "block_comment" {
            count += child.end_position().row - child.start_position().row + 1;
        }
    }
    // Also recurse into body
    if let Some(body) = node.child_by_field_name("body") {
        count += count_comment_lines_recursive(body, src);
    }
    count
}

fn count_comment_lines_recursive(node: Node, _src: &[u8]) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "line_comment" || child.kind() == "block_comment" {
            count += child.end_position().row - child.start_position().row + 1;
        } else if child.child_count() > 0 {
            count += count_comment_lines_recursive(child, _src);
        }
    }
    count
}

// cha:ignore todo_comment
/// Collect field references (self.xxx) from a function body.
fn collect_field_refs(body: Option<Node>, src: &[u8]) -> Vec<String> {
    let Some(body) = body else { return vec![] };
    let mut refs = Vec::new();
    collect_self_fields(body, src, &mut refs);
    refs.sort();
    refs.dedup();
    refs
}

fn collect_self_fields(node: Node, src: &[u8], refs: &mut Vec<String>) {
    if node.kind() == "field_expression"
        && let Some(obj) = node.child_by_field_name("value")
        && node_text(obj, src) == "self"
        && let Some(field) = node.child_by_field_name("field")
    {
        refs.push(node_text(field, src).to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_self_fields(child, src, refs);
    }
}

/// Extract field names from a struct body.
/// Returns (field_count, field_names, callback_collection_field_names).
fn extract_fields(node: Node, src: &[u8]) -> (usize, Vec<String>, Vec<String>) {
    let mut names = Vec::new();
    let mut callback_fields = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration"
                && let Some(name_node) = child.child_by_field_name("name")
            {
                let name = node_text(name_node, src).to_string();
                if let Some(ty) = child.child_by_field_name("type")
                    && is_callback_collection_type_rs(node_text(ty, src))
                {
                    callback_fields.push(name.clone());
                }
                names.push(name);
            }
        }
    }
    (names.len(), names, callback_fields)
}

/// Check if a type looks like a collection of callbacks: Vec<Box<dyn Fn*>>, Vec<fn(...)>.
fn is_callback_collection_type_rs(ty: &str) -> bool {
    if !ty.contains("Vec<") {
        return false;
    }
    ty.contains("Fn(") || ty.contains("FnMut(") || ty.contains("FnOnce(") || ty.contains("fn(")
}

/// Collect field names checked for None/null in match/if-let patterns.
fn collect_null_checks(body: Option<Node>, src: &[u8]) -> Vec<String> {
    let Some(body) = body else { return vec![] };
    let mut fields = Vec::new();
    walk_null_checks_rs(body, src, &mut fields);
    fields.sort();
    fields.dedup();
    fields
}

fn walk_null_checks_rs(node: Node, src: &[u8], fields: &mut Vec<String>) {
    if node.kind() == "if_let_expression" {
        // if let Some(x) = self.field { ... }
        if let Some(pattern) = node.child_by_field_name("pattern")
            && node_text(pattern, src).contains("Some")
            && let Some(value) = node.child_by_field_name("value")
        {
            let vtext = node_text(value, src);
            if let Some(f) = vtext.strip_prefix("self.") {
                fields.push(f.to_string());
            }
        }
    } else if node.kind() == "if_expression"
        && let Some(cond) = node.child_by_field_name("condition")
    {
        let text = node_text(cond, src);
        if text.contains("is_some") || text.contains("is_none") {
            extract_null_checked_fields(text, fields);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_null_checks_rs(child, src, fields);
    }
}

/// Extract self.field names from a condition text containing null checks.
fn extract_null_checked_fields(text: &str, fields: &mut Vec<String>) {
    if !(text.contains("is_some") || text.contains("is_none") || text.contains("Some")) {
        return;
    }
    for part in text.split("self.") {
        if let Some(field) = part
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            && !field.is_empty()
            && field != "is_some"
            && field != "is_none"
        {
            fields.push(field.to_string());
        }
    }
}

/// Extract the variable/field being dispatched on in a match expression.
fn extract_switch_target(body: Option<Node>, src: &[u8]) -> Option<String> {
    let body = body?;
    find_match_target(body, src)
}

fn find_match_target(node: Node, src: &[u8]) -> Option<String> {
    if node.kind() == "match_expression"
        && let Some(value) = node.child_by_field_name("value")
    {
        return Some(node_text(value, src).to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(t) = find_match_target(child, src) {
            return Some(t);
        }
    }
    None
}

/// Count optional parameters (those with default values or Option<T> type).
fn count_optional_params(node: Node, src: &[u8]) -> usize {
    let Some(params) = node.child_by_field_name("parameters") else {
        return 0;
    };
    let mut count = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() == "parameter" {
            let text = node_text(child, src);
            if text.contains("Option<") {
                count += 1;
            }
        }
    }
    count
}

/// Count self.method() calls in a function body (for Template Method detection).
fn count_self_method_calls(body: Option<Node>, src: &[u8]) -> usize {
    let Some(body) = body else { return 0 };
    let mut count = 0;
    walk_self_calls(body, src, &mut count);
    count
}

fn walk_self_calls(node: Node, src: &[u8], count: &mut usize) {
    if node.kind() == "call_expression"
        && let Some(func) = node.child_by_field_name("function")
        && node_text(func, src).starts_with("self.")
    {
        *count += 1;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_self_calls(child, src, count);
    }
}

/// Structural Observer detection: method body iterates a callback collection field and calls elements.
/// Pattern: `for cb in &self.field { cb(...) }` or `self.field.iter().for_each(|cb| cb(...))`
fn has_iterate_and_call(body: Option<Node>, src: &[u8], cb_fields: &[String]) -> bool {
    let Some(body) = body else { return false };
    for field in cb_fields {
        let self_field = format!("self.{field}");
        if walk_for_iterate_call(body, src, &self_field) {
            return true;
        }
    }
    false
}

fn walk_for_iterate_call(node: Node, src: &[u8], self_field: &str) -> bool {
    // for x in &self.field { x(...) }
    if node.kind() == "for_expression"
        && let Some(value) = node.child_by_field_name("value")
        && node_text(value, src).contains(self_field)
        && let Some(loop_body) = node.child_by_field_name("body")
        && has_call_expression(loop_body)
    {
        return true;
    }
    // self.field.iter().for_each(|cb| cb(...))
    if node.kind() == "call_expression" {
        let text = node_text(node, src);
        if text.contains(self_field) && text.contains("for_each") {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if walk_for_iterate_call(child, src, self_field) {
            return true;
        }
    }
    false
}

fn cognitive_complexity_rs(node: Node) -> usize {
    let mut score = 0;
    cc_walk_rs(node, 0, &mut score);
    score
}

fn cc_walk_rs(node: Node, nesting: usize, score: &mut usize) {
    match node.kind() {
        "if_expression" => {
            *score += 1 + nesting;
            cc_children_rs(node, nesting + 1, score);
            return;
        }
        "for_expression" | "while_expression" | "loop_expression" => {
            *score += 1 + nesting;
            cc_children_rs(node, nesting + 1, score);
            return;
        }
        "match_expression" => {
            *score += 1 + nesting;
            cc_children_rs(node, nesting + 1, score);
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
        "closure_expression" => {
            cc_children_rs(node, nesting + 1, score);
            return;
        }
        _ => {}
    }
    cc_children_rs(node, nesting, score);
}

fn cc_children_rs(node: Node, nesting: usize, score: &mut usize) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        cc_walk_rs(child, nesting, score);
    }
}

fn collect_calls_rs(body: Option<tree_sitter::Node>, src: &[u8]) -> Vec<String> {
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
            visit_all(cursor.node(), cursor, f);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
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

fn has_call_expression(node: Node) -> bool {
    if node.kind() == "call_expression" {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_call_expression(child) {
            return true;
        }
    }
    false
}
