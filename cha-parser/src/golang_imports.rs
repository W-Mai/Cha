//! Go `import` → ImportsMap resolver.
//!
//! Go imports bind *package* names, not type names. Types are referenced as
//! `pkg.Type`, so the ImportsMap key is the package alias (default last
//! segment of the path, or explicit alias).
//!
//! To classify a package as Local vs External we try to parse `go.mod` up
//! the directory tree for the project's `module` path; anything that starts
//! with that path is Local, anything else is External.

use cha_core::TypeOrigin;
use std::path::Path;
use tree_sitter::Node;

use crate::type_ref::ImportsMap;

pub fn build(root: Node, src: &[u8], file_path: &Path) -> ImportsMap {
    let module_path = find_module_path(file_path);
    let mut map = ImportsMap::new();
    walk(root, src, module_path.as_deref(), &mut map);
    map
}

fn walk(node: Node, src: &[u8], module: Option<&str>, map: &mut ImportsMap) {
    if node.kind() == "import_spec" {
        process_spec(node, src, module, map);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, module, map);
    }
}

fn process_spec(node: Node, src: &[u8], module: Option<&str>, map: &mut ImportsMap) {
    let path_node = node
        .child_by_field_name("path")
        .or_else(|| find_child(node, "interpreted_string_literal"))
        .or_else(|| find_child(node, "raw_string_literal"));
    let Some(path_node) = path_node else {
        return;
    };
    let raw = path_node.utf8_text(src).unwrap_or("");
    let path = raw.trim_matches(|c| c == '"' || c == '`');
    if path.is_empty() {
        return;
    }
    let alias = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string);
    let short = alias.unwrap_or_else(|| path.rsplit('/').next().unwrap_or(path).to_string());
    if short == "_" || short == "." {
        return;
    }
    map.insert(short, classify_path(path, module));
}

fn find_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

fn classify_path(path: &str, module: Option<&str>) -> TypeOrigin {
    if let Some(m) = module
        && (path == m || path.starts_with(&format!("{m}/")))
    {
        return TypeOrigin::Local;
    }
    // Go stdlib packages have no "." in the path (e.g. "fmt", "strings", "net/http").
    // Third-party packages have a "." (e.g. "github.com/...", "golang.org/x/...").
    let first_seg = path.split('/').next().unwrap_or(path);
    if !first_seg.contains('.') {
        return TypeOrigin::Primitive;
    }
    TypeOrigin::External(path.to_string())
}

/// Walk up from `file_path`'s directory looking for `go.mod` and parse the
/// `module <path>` directive.
fn find_module_path(file_path: &Path) -> Option<String> {
    let mut dir = file_path.parent()?.to_path_buf();
    loop {
        let candidate = dir.join("go.mod");
        if let Ok(content) = std::fs::read_to_string(&candidate) {
            for line in content.lines() {
                if let Some(rest) = line.trim().strip_prefix("module ") {
                    return Some(rest.trim().to_string());
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_with_module(src: &str, module: Option<&str>) -> ImportsMap {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(src, None).unwrap();
        let mut map = ImportsMap::new();
        walk(tree.root_node(), src.as_bytes(), module, &mut map);
        map
    }

    #[test]
    fn stdlib_package() {
        let m = parse_with_module("package p\nimport \"fmt\"\n", None);
        assert_eq!(m.get("fmt"), Some(&TypeOrigin::Primitive));
    }

    #[test]
    fn stdlib_multi_segment() {
        let m = parse_with_module("package p\nimport \"net/http\"\n", None);
        assert_eq!(m.get("http"), Some(&TypeOrigin::Primitive));
    }

    #[test]
    fn third_party() {
        let m = parse_with_module("package p\nimport \"github.com/foo/bar\"\n", None);
        assert_eq!(
            m.get("bar"),
            Some(&TypeOrigin::External("github.com/foo/bar".into()))
        );
    }

    #[test]
    fn aliased_third_party() {
        let m = parse_with_module("package p\nimport bar \"github.com/foo/bar\"\n", None);
        assert_eq!(
            m.get("bar"),
            Some(&TypeOrigin::External("github.com/foo/bar".into()))
        );
    }

    #[test]
    fn project_local_via_module() {
        let m = parse_with_module("package p\nimport \"myapp/models\"\n", Some("myapp"));
        assert_eq!(m.get("models"), Some(&TypeOrigin::Local));
    }

    #[test]
    fn grouped_imports() {
        let m = parse_with_module(
            "package p\nimport (\n  \"fmt\"\n  \"github.com/foo/bar\"\n)\n",
            None,
        );
        assert_eq!(m.get("fmt"), Some(&TypeOrigin::Primitive));
        assert_eq!(
            m.get("bar"),
            Some(&TypeOrigin::External("github.com/foo/bar".into()))
        );
    }
}
