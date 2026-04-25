//! Rust `use` declaration → ImportsMap resolver.

use cha_core::{ImportInfo, TypeOrigin, TypeRef};
use tree_sitter::Node;

use crate::type_ref::{self, ImportsMap};

/// Read the `return_type` field of a `function_item`, resolving the origin
/// via the file's imports map.
pub fn rust_return_type(node: Node, src: &[u8], imports: &ImportsMap) -> Option<TypeRef> {
    let rt = node.child_by_field_name("return_type")?;
    let raw = rt.utf8_text(src).unwrap_or("").to_string();
    Some(type_ref::resolve(raw, imports))
}

/// Build an `ImportInfo` from a `use_declaration` node.
pub fn extract_use(node: Node, src: &[u8]) -> Option<ImportInfo> {
    let source = node
        .utf8_text(src)
        .unwrap_or("")
        .strip_prefix("use ")?
        .trim_end_matches(';')
        .trim()
        .to_string();
    Some(ImportInfo {
        source,
        line: node.start_position().row + 1,
        col: node.start_position().column,
        ..Default::default()
    })
}

/// Scan the whole tree for `use` declarations and build a short-name ->
/// TypeOrigin map. Call once per file before collecting functions.
pub fn build(root: Node, src: &[u8]) -> ImportsMap {
    let mut map = ImportsMap::new();
    walk(root, src, &mut map);
    map
}

fn walk(node: Node, src: &[u8], map: &mut ImportsMap) {
    if node.kind() == "use_declaration" {
        let text = node.utf8_text(src).unwrap_or("").trim();
        if let Some(rest) = text.strip_prefix("use ") {
            let path = rest.trim_end_matches(';').trim();
            parse_use_path(path, map);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, map);
    }
}

/// Parse one `use` path and insert all short-name bindings into map.
/// Handles `foo::Bar`, `foo::Bar as Baz`, `foo::{Bar, Baz}`.
fn parse_use_path(path: &str, map: &mut ImportsMap) {
    // Group expansion: `foo::{A, B as C}`.
    if let Some(open) = path.find('{')
        && let Some(close) = path.rfind('}')
    {
        let prefix = path[..open].trim_end_matches("::").trim();
        for item in path[open + 1..close].split(',') {
            let item = item.trim();
            if !item.is_empty() {
                parse_use_path(&format!("{prefix}::{item}"), map);
            }
        }
        return;
    }
    let (path_part, alias) = match path.split_once(" as ") {
        Some((p, a)) => (p.trim(), Some(a.trim().to_string())),
        None => (path, None),
    };
    if path_part.ends_with("::*") || path_part == "*" {
        return;
    }
    let short = alias.unwrap_or_else(|| {
        path_part
            .rsplit("::")
            .next()
            .unwrap_or(path_part)
            .to_string()
    });
    map.insert(short, classify(path_part));
}

fn classify(path: &str) -> TypeOrigin {
    let root = path.split("::").next().unwrap_or("").trim();
    match root {
        "crate" | "self" | "super" => TypeOrigin::Local,
        "std" | "core" | "alloc" => TypeOrigin::Primitive,
        "" => TypeOrigin::Unknown,
        other => TypeOrigin::External(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ImportsMap {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(src, None).unwrap();
        build(tree.root_node(), src.as_bytes())
    }

    #[test]
    fn simple_external() {
        let m = parse("use tree_sitter::Node;");
        assert_eq!(
            m.get("Node"),
            Some(&TypeOrigin::External("tree_sitter".into()))
        );
    }

    #[test]
    fn crate_local() {
        let m = parse("use crate::model::Finding;");
        assert_eq!(m.get("Finding"), Some(&TypeOrigin::Local));
    }

    #[test]
    fn std_primitive() {
        let m = parse("use std::collections::HashMap;");
        assert_eq!(m.get("HashMap"), Some(&TypeOrigin::Primitive));
    }

    #[test]
    fn alias_rename() {
        let m = parse("use tree_sitter::Node as TsNode;");
        assert_eq!(
            m.get("TsNode"),
            Some(&TypeOrigin::External("tree_sitter".into()))
        );
        assert!(m.get("Node").is_none());
    }

    #[test]
    fn group_expansion() {
        let m = parse("use tree_sitter::{Node, Parser};");
        assert_eq!(
            m.get("Node"),
            Some(&TypeOrigin::External("tree_sitter".into()))
        );
        assert_eq!(
            m.get("Parser"),
            Some(&TypeOrigin::External("tree_sitter".into()))
        );
    }

    #[test]
    fn glob_ignored() {
        let m = parse("use tree_sitter::*;");
        assert!(m.is_empty());
    }
}
