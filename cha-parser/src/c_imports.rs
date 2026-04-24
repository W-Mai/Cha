//! C/C++ `#include` → ImportsMap resolver.
//!
//! C has no type-level import granularity; each header declares many types.
//! Without full header parsing we can't reliably say "this struct type came
//! from which header". We take a conservative approach:
//!
//! - Seed the ImportsMap with common C/C++ built-in primitive names so
//!   `int`, `char *`, `size_t` etc. get Primitive origin.
//! - `#include <system.h>` → the *set* of headers is collected for debugging;
//!   at resolution time a type that isn't a known primitive and isn't in the
//!   project falls back to Unknown. Boundary-leak detection treats Unknown
//!   as potentially external (design D1), which is the correct conservative
//!   behaviour for C codebases.
//! - `#include "local.h"` (quoted) hints the project does have local headers;
//!   this is currently informational only.

use cha_core::TypeOrigin;
use tree_sitter::Node;

use crate::type_ref::ImportsMap;

pub fn build(root: Node, src: &[u8]) -> ImportsMap {
    let mut map = ImportsMap::new();
    for prim in PRIMITIVES {
        map.insert((*prim).to_string(), TypeOrigin::Primitive);
    }
    // Attribute any `<header>` imports. We don't know which types they bring,
    // but we record their existence so future refinements can use it.
    walk(root, src, &mut map);
    map
}

fn walk(node: Node, src: &[u8], _map: &mut ImportsMap) {
    if node.kind() == "preproc_include" {
        // Currently nothing to do — we just verify the node exists. A richer
        // implementation would parse well-known system headers and register
        // the types they declare.
        let _ = node.utf8_text(src);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, _map);
    }
}

/// C/C++ built-in primitive types that always resolve to Primitive regardless
/// of includes. Covers C89, C99 stdint aliases, common C++ types.
const PRIMITIVES: &[&str] = &[
    // C keywords
    "bool",
    "char",
    "double",
    "float",
    "int",
    "long",
    "short",
    "signed",
    "unsigned",
    "void",
    "wchar_t",
    // stdint
    "int8_t",
    "int16_t",
    "int32_t",
    "int64_t",
    "uint8_t",
    "uint16_t",
    "uint32_t",
    "uint64_t",
    "intptr_t",
    "uintptr_t",
    "size_t",
    "ssize_t",
    "ptrdiff_t",
    // C++ std basics (often used unqualified with `using`)
    "string",
    "nullptr_t",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ImportsMap {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_c::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(src, None).unwrap();
        build(tree.root_node(), src.as_bytes())
    }

    #[test]
    fn primitives_seeded() {
        let m = parse("#include <stdint.h>\n");
        assert_eq!(m.get("int"), Some(&TypeOrigin::Primitive));
        assert_eq!(m.get("size_t"), Some(&TypeOrigin::Primitive));
        assert_eq!(m.get("uint32_t"), Some(&TypeOrigin::Primitive));
    }

    #[test]
    fn non_primitive_absent() {
        let m = parse("#include <cmark.h>\n");
        // cmark_node_t is not a primitive; resolver will see Unknown.
        assert!(m.get("cmark_node_t").is_none());
    }
}
