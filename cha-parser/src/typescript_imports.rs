//! TypeScript `import` → ImportsMap resolver.
//!
//! Relative paths (`./x`, `../x`) → Local. Bare specifiers → External(pkg).
//! Supports named, default, namespace, and rename forms.

use cha_core::TypeOrigin;
use tree_sitter::Node;

use crate::type_ref::ImportsMap;

pub fn build(root: Node, src: &[u8]) -> ImportsMap {
    let mut map = ImportsMap::new();
    walk(root, src, &mut map);
    map
}

fn walk(node: Node, src: &[u8], map: &mut ImportsMap) {
    if node.kind() == "import_statement" {
        process_import(node, src, map);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, map);
    }
}

fn process_import(node: Node, src: &[u8], map: &mut ImportsMap) {
    let Some(source) = find_source(node, src) else {
        return;
    };
    let origin = classify(&source);
    add_bindings(node, src, &origin, map);
}

fn find_source(node: Node, src: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string" {
            return Some(
                child
                    .utf8_text(src)
                    .unwrap_or("")
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string(),
            );
        }
    }
    None
}

fn add_bindings(node: Node, src: &[u8], origin: &TypeOrigin, map: &mut ImportsMap) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => add_identifier(child, src, origin, map),
            "import_clause" => add_from_clause(child, src, origin, map),
            _ => {}
        }
    }
}

fn add_from_clause(clause: Node, src: &[u8], origin: &TypeOrigin, map: &mut ImportsMap) {
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        match child.kind() {
            "identifier" => add_identifier(child, src, origin, map),
            "named_imports" => add_named_imports(child, src, origin, map),
            "namespace_import" => add_namespace_import(child, src, origin, map),
            _ => {}
        }
    }
}

fn add_identifier(node: Node, src: &[u8], origin: &TypeOrigin, map: &mut ImportsMap) {
    let short = node.utf8_text(src).unwrap_or("").to_string();
    if !short.is_empty() {
        map.insert(short, origin.clone());
    }
}

fn add_namespace_import(node: Node, src: &[u8], origin: &TypeOrigin, map: &mut ImportsMap) {
    let mut cursor = node.walk();
    for c in node.children(&mut cursor) {
        if c.kind() == "identifier" {
            add_identifier(c, src, origin, map);
        }
    }
}

fn add_named_imports(named: Node, src: &[u8], origin: &TypeOrigin, map: &mut ImportsMap) {
    let mut cursor = named.walk();
    for spec in named.children(&mut cursor) {
        if spec.kind() != "import_specifier" {
            continue;
        }
        let name = spec
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok())
            .unwrap_or("");
        let alias = spec
            .child_by_field_name("alias")
            .and_then(|n| n.utf8_text(src).ok());
        let short = alias.unwrap_or(name);
        if !short.is_empty() {
            map.insert(short.to_string(), origin.clone());
        }
    }
}

fn classify(source: &str) -> TypeOrigin {
    if source.starts_with("./") || source.starts_with("../") || source.starts_with('/') {
        return TypeOrigin::Local;
    }
    let module = if source.starts_with('@') {
        source.splitn(3, '/').take(2).collect::<Vec<_>>().join("/")
    } else {
        source.split('/').next().unwrap_or(source).to_string()
    };
    TypeOrigin::External(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ImportsMap {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(src, None).unwrap();
        build(tree.root_node(), src.as_bytes())
    }

    #[test]
    fn named_external() {
        let m = parse("import { Foo } from 'pkg';");
        assert_eq!(m.get("Foo"), Some(&TypeOrigin::External("pkg".into())));
    }

    #[test]
    fn rename_alias() {
        let m = parse("import { Foo as Bar } from 'pkg';");
        assert_eq!(m.get("Bar"), Some(&TypeOrigin::External("pkg".into())));
        assert!(m.get("Foo").is_none());
    }

    #[test]
    fn default_external() {
        let m = parse("import Foo from 'pkg';");
        assert_eq!(m.get("Foo"), Some(&TypeOrigin::External("pkg".into())));
    }

    #[test]
    fn namespace_external() {
        let m = parse("import * as Pkg from 'pkg';");
        assert_eq!(m.get("Pkg"), Some(&TypeOrigin::External("pkg".into())));
    }

    #[test]
    fn relative_local() {
        let m = parse("import { Helper } from './helper';");
        assert_eq!(m.get("Helper"), Some(&TypeOrigin::Local));
    }

    #[test]
    fn scoped_package() {
        let m = parse("import { Foo } from '@scope/pkg';");
        assert_eq!(
            m.get("Foo"),
            Some(&TypeOrigin::External("@scope/pkg".into()))
        );
    }
}
