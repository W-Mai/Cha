//! Python `import` / `from ... import ...` → ImportsMap resolver.

use cha_core::TypeOrigin;
use tree_sitter::Node;

use crate::type_ref::ImportsMap;

pub fn build(root: Node, src: &[u8]) -> ImportsMap {
    let mut map = ImportsMap::new();
    walk(root, src, &mut map);
    map
}

fn walk(node: Node, src: &[u8], map: &mut ImportsMap) {
    match node.kind() {
        "import_statement" => process_plain(node, src, map),
        "import_from_statement" => process_from(node, src, map),
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, map);
    }
}

/// `import foo`, `import foo.bar`, `import foo as bar`
fn process_plain(node: Node, src: &[u8], map: &mut ImportsMap) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "dotted_name" => {
                let full = child.utf8_text(src).unwrap_or("");
                let root_mod = full.split('.').next().unwrap_or("").to_string();
                if !root_mod.is_empty() {
                    map.insert(root_mod.clone(), classify_module(&root_mod));
                }
            }
            "aliased_import" => {
                let path = child
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(src).ok())
                    .unwrap_or("");
                let alias = child
                    .child_by_field_name("alias")
                    .and_then(|n| n.utf8_text(src).ok())
                    .unwrap_or("");
                let root_mod = path.split('.').next().unwrap_or("");
                if !alias.is_empty() && !root_mod.is_empty() {
                    map.insert(alias.to_string(), classify_module(root_mod));
                }
            }
            _ => {}
        }
    }
}

/// `from foo import Bar`, `from foo import Bar as Baz`, `from .foo import Bar`
fn process_from(node: Node, src: &[u8], map: &mut ImportsMap) {
    let module = node
        .child_by_field_name("module_name")
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or("");
    let origin = classify_module(module);
    // Walk all aliased_import / dotted_name children after 'module_name'.
    let mut cursor = node.walk();
    let mut saw_module = false;
    for child in node.children(&mut cursor) {
        if Some(child) == node.child_by_field_name("module_name") {
            saw_module = true;
            continue;
        }
        if !saw_module {
            continue;
        }
        match child.kind() {
            "dotted_name" => {
                let name = child.utf8_text(src).unwrap_or("");
                // For `from foo import bar.baz`, only the top segment is bound.
                let short = name.split('.').next().unwrap_or("");
                if !short.is_empty() {
                    map.insert(short.to_string(), origin.clone());
                }
            }
            "aliased_import" => {
                let alias = child
                    .child_by_field_name("alias")
                    .and_then(|n| n.utf8_text(src).ok())
                    .unwrap_or("");
                if !alias.is_empty() {
                    map.insert(alias.to_string(), origin.clone());
                }
            }
            _ => {}
        }
    }
}

const STDLIB_MODULES: &[&str] = &[
    "__future__",
    "abc",
    "argparse",
    "array",
    "asyncio",
    "base64",
    "collections",
    "concurrent",
    "contextlib",
    "copy",
    "csv",
    "dataclasses",
    "datetime",
    "enum",
    "functools",
    "hashlib",
    "http",
    "io",
    "itertools",
    "json",
    "logging",
    "math",
    "os",
    "pathlib",
    "queue",
    "random",
    "re",
    "socket",
    "string",
    "struct",
    "subprocess",
    "sys",
    "threading",
    "time",
    "traceback",
    "types",
    "typing",
    "unittest",
    "urllib",
    "uuid",
    "warnings",
    "weakref",
];

fn classify_module(module: &str) -> TypeOrigin {
    if module.starts_with('.') {
        return TypeOrigin::Local;
    }
    let root = module.split('.').next().unwrap_or("");
    if STDLIB_MODULES.contains(&root) {
        return TypeOrigin::Primitive;
    }
    if root.is_empty() {
        return TypeOrigin::Unknown;
    }
    TypeOrigin::External(root.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ImportsMap {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(src, None).unwrap();
        build(tree.root_node(), src.as_bytes())
    }

    #[test]
    fn from_external_named() {
        let m = parse("from pydantic import BaseModel");
        assert_eq!(
            m.get("BaseModel"),
            Some(&TypeOrigin::External("pydantic".into()))
        );
    }

    #[test]
    fn from_alias() {
        let m = parse("from pydantic import BaseModel as BM");
        assert_eq!(m.get("BM"), Some(&TypeOrigin::External("pydantic".into())));
    }

    #[test]
    fn from_relative_local() {
        let m = parse("from .models import User");
        assert_eq!(m.get("User"), Some(&TypeOrigin::Local));
    }

    #[test]
    fn from_stdlib_primitive() {
        let m = parse("from typing import List");
        assert_eq!(m.get("List"), Some(&TypeOrigin::Primitive));
    }

    #[test]
    fn plain_import_alias() {
        let m = parse("import numpy as np");
        assert_eq!(m.get("np"), Some(&TypeOrigin::External("numpy".into())));
    }

    #[test]
    fn plain_import_stdlib() {
        let m = parse("import os");
        assert_eq!(m.get("os"), Some(&TypeOrigin::Primitive));
    }
}
