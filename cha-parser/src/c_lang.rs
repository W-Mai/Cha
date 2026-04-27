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
    let mut type_aliases = Vec::new();

    let imports_map = crate::c_imports::build(root, src);
    collect_top_level(
        root,
        src,
        &imports_map,
        &mut functions,
        &mut classes,
        &mut imports,
        &mut type_aliases,
    );

    // (C OOP method attribution moved to `cha-cli::c_oop_enrich`, which has
    // cross-file visibility — a function in `foo.c` can be attributed to a
    // struct declared in `foo.h`.)

    if is_header_file(file) {
        for f in &mut functions {
            f.is_exported = true;
        }
    }

    Some(SourceModel {
        language: lang.into(),
        total_lines: file.line_count(),
        functions,
        classes,
        imports,
        comments: collect_comments(root, src),
        type_aliases,
    })
}

fn is_header_file(file: &SourceFile) -> bool {
    file.path
        .extension()
        .is_some_and(|e| e == "h" || e == "hxx" || e == "hpp")
}

// cha:ignore cognitive_complexity
fn collect_top_level(
    root: Node,
    src: &[u8],
    imports_map: &crate::type_ref::ImportsMap,
    functions: &mut Vec<FunctionInfo>,
    classes: &mut Vec<ClassInfo>,
    imports: &mut Vec<ImportInfo>,
    type_aliases: &mut Vec<(String, String)>,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                // Heuristic: `class MACRO Name { ... };` is parsed by tree-sitter
                // as a function_definition whose return type is a class_specifier.
                // Detect this and extract as a class instead.
                if let Some(c) = try_extract_macro_class(child, src) {
                    classes.push(c);
                } else if let Some(f) = extract_function(child, src, imports_map) {
                    functions.push(f);
                }
            }
            "declaration" => {
                // Header-style function declarations (`void foo(int);` — no
                // body) surface as `declaration` nodes in tree-sitter-c. Pick
                // out the function_declarator variants and treat them as
                // functions so `.h` file contents contribute to the project
                // API surface / method attribution. Non-function
                // `declaration` (globals, typedefs, extern vars) have no
                // function_declarator child and are skipped.
                if has_function_declarator(child)
                    && let Some(f) = extract_function(child, src, imports_map)
                {
                    functions.push(f);
                }
            }
            "struct_specifier" | "class_specifier" => {
                if let Some(c) = extract_class(child, src) {
                    classes.push(c);
                }
            }
            "type_definition" => {
                extract_typedef_struct(child, src, classes, type_aliases);
            }
            "preproc_include" => {
                if let Some(imp) = extract_include(child, src) {
                    imports.push(imp);
                }
            }
            _ => {
                if child.child_count() > 0 {
                    collect_top_level(
                        child,
                        src,
                        imports_map,
                        functions,
                        classes,
                        imports,
                        type_aliases,
                    );
                }
            }
        }
    }
}

fn extract_typedef_struct(
    node: Node,
    src: &[u8],
    classes: &mut Vec<ClassInfo>,
    type_aliases: &mut Vec<(String, String)>,
) {
    let found_struct = register_typedef_struct_children(node, src, classes, type_aliases);
    if !found_struct {
        register_simple_typedef(node, src, type_aliases);
    }
}

fn register_typedef_struct_children(
    node: Node,
    src: &[u8],
    classes: &mut Vec<ClassInfo>,
    type_aliases: &mut Vec<(String, String)>,
) -> bool {
    let mut found_struct = false;
    let mut inner = node.walk();
    for sub in node.children(&mut inner) {
        if sub.kind() != "struct_specifier" && sub.kind() != "class_specifier" {
            continue;
        }
        found_struct = true;
        register_single_typedef_struct(node, sub, src, classes, type_aliases);
    }
    found_struct
}

fn register_single_typedef_struct(
    typedef: Node,
    sub: Node,
    src: &[u8],
    classes: &mut Vec<ClassInfo>,
    type_aliases: &mut Vec<(String, String)>,
) {
    let Some(mut c) = extract_class(sub, src) else {
        return;
    };
    let original_name = c.name.clone();
    if c.name.is_empty()
        && let Some(decl) = typedef.child_by_field_name("declarator")
    {
        c.name = node_text(decl, src).to_string();
    }
    if !original_name.is_empty()
        && let Some(decl) = typedef.child_by_field_name("declarator")
    {
        let alias = node_text(decl, src).to_string();
        if alias != original_name {
            type_aliases.push((alias, original_name));
        }
    }
    if !c.name.is_empty() {
        classes.push(c);
    }
}

/// `typedef uint32_t tag_t;` — simple alias, no struct body.
fn register_simple_typedef(node: Node, src: &[u8], type_aliases: &mut Vec<(String, String)>) {
    let alias = extract_typedef_alias(node, src);
    let original = node
        .child_by_field_name("type")
        .map(|t| node_text(t, src).trim().to_string())
        .unwrap_or_default();
    if !alias.is_empty() && alias != original {
        type_aliases.push((alias, original));
    }
}

/// Find the new type name in a `typedef <something> <name>;`. Tree-sitter-c
/// sometimes puts the name behind the `declarator` field; other grammars
/// (typedef of enum/union without body) emit a plain `type_identifier` as
/// a top-level child. Try the field first, then the first type_identifier.
fn extract_typedef_alias(node: Node, src: &[u8]) -> String {
    if let Some(decl) = node.child_by_field_name("declarator") {
        return node_text(decl, src).trim().to_string();
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            return node_text(child, src).trim().to_string();
        }
    }
    String::new()
}

/// Detect `class MACRO ClassName { ... };` misparse.
/// tree-sitter sees this as function_definition with class_specifier return type.
fn try_extract_macro_class(node: Node, src: &[u8]) -> Option<ClassInfo> {
    let mut has_class_spec = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_specifier" || child.kind() == "struct_specifier" {
            has_class_spec = true;
        }
    }
    if !has_class_spec {
        return None;
    }
    // The real class name is the "identifier" child (what tree-sitter thinks is the func name)
    let name_node = node
        .child_by_field_name("declarator")
        .filter(|d| d.kind() == "identifier")?;
    let name = node_text(name_node, src).to_string();
    let name_col = name_node.start_position().column;
    let name_end_col = name_node.end_position().column;
    let body = node.child_by_field_name("body")?;
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let method_count = count_methods(body);
    let (field_names, field_types, first_field_type) = extract_field_info(body, src);

    // Find parent from the class_specifier's base_class_clause if present
    let parent_name = first_field_type;

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        name_col,
        name_end_col,
        line_count: end_line - start_line + 1,
        method_count,
        is_exported: true,
        delegating_method_count: 0,
        field_count: field_names.len(),
        field_names,
        field_types,
        has_behavior: method_count > 0,
        is_interface: false,
        parent_name,
        override_count: 0,
        self_call_count: 0,
        has_listener_field: false,
        has_notify_method: false,
    })
}

fn extract_function(
    node: Node,
    src: &[u8],
    imports_map: &crate::type_ref::ImportsMap,
) -> Option<FunctionInfo> {
    let declarator = node.child_by_field_name("declarator")?;
    let name_node = find_func_name_node(declarator)?;
    let name = node_text(name_node, src).to_string();
    let name_col = name_node.start_position().column;
    let name_end_col = name_node.end_position().column;
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let body = node.child_by_field_name("body");
    let (param_count, param_types) = extract_params(declarator, src, imports_map);
    let is_static = has_storage_class(node, src, "static");

    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        name_col,
        name_end_col,
        line_count: end_line - start_line + 1,
        complexity: count_complexity(node),
        body_hash: body.map(hash_ast),
        is_exported: !is_static,
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
        return_type: extract_c_return_type(node, src, imports_map),
    })
}

/// The C function's return type lives in the `type` field of the
/// `function_definition` node. Pointer return types are indicated by the
/// declarator having a `pointer_declarator`; prefix the type with ` *`
/// so the `raw` text mirrors the written form.
fn extract_c_return_type(
    node: Node,
    src: &[u8],
    imports_map: &crate::type_ref::ImportsMap,
) -> Option<cha_core::TypeRef> {
    let ty = node.child_by_field_name("type")?;
    let base = node_text(ty, src).trim().to_string();
    let is_ptr = node
        .child_by_field_name("declarator")
        .is_some_and(|d| d.kind() == "pointer_declarator");
    let raw = if is_ptr { format!("{base} *") } else { base };
    Some(crate::type_ref::resolve(raw, imports_map))
}

/// Does this `declaration` node actually declare a function (as opposed
/// to a variable / typedef / extern)? tree-sitter-c wraps function
/// prototypes in `declaration` with a `function_declarator` descendant.
fn has_function_declarator(node: Node) -> bool {
    node.child_by_field_name("declarator")
        .is_some_and(has_function_declarator_inside)
}

fn has_function_declarator_inside(node: Node) -> bool {
    if node.kind() == "function_declarator" {
        return true;
    }
    // Pointer return types wrap the declarator: `int *foo(...)` produces
    // `pointer_declarator { function_declarator { ... } }`. Unwrap.
    if let Some(inner) = node.child_by_field_name("declarator") {
        return has_function_declarator_inside(inner);
    }
    false
}

/// Check if a declaration node has a specific storage class specifier (e.g. "static").
fn has_storage_class(node: Node, src: &[u8], keyword: &str) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i)
            && child.kind() == "storage_class_specifier"
            && node_text(child, src) == keyword
        {
            return true;
        }
    }
    false
}

fn find_func_name_node(declarator: Node) -> Option<Node> {
    if declarator.kind() == "identifier" {
        return Some(declarator);
    }
    declarator
        .child_by_field_name("declarator")
        .and_then(find_func_name_node)
}

fn extract_params(
    declarator: Node,
    src: &[u8],
    imports_map: &crate::type_ref::ImportsMap,
) -> (usize, Vec<cha_core::TypeRef>) {
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
            let base = child
                .child_by_field_name("type")
                .map(|t| node_text(t, src).to_string())
                .unwrap_or_else(|| "int".into());
            let is_ptr = child
                .child_by_field_name("declarator")
                .is_some_and(|d| d.kind() == "pointer_declarator");
            let raw = if is_ptr { format!("{base} *") } else { base };
            types.push(crate::type_ref::resolve(raw, imports_map));
        }
    }
    (count, types)
}

fn extract_class(node: Node, src: &[u8]) -> Option<ClassInfo> {
    let name_node = node.child_by_field_name("name");
    let name = name_node
        .map(|n| node_text(n, src).to_string())
        .unwrap_or_default();
    let name_col = name_node.map(|n| n.start_position().column).unwrap_or(0);
    let name_end_col = name_node.map(|n| n.end_position().column).unwrap_or(0);
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
        name_col,
        name_end_col,
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
        col: node.start_position().column,
        ..Default::default()
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
