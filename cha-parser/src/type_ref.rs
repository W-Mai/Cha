//! TypeRef construction and import resolution helpers shared across
//! language parsers.
//!
//! Each parser walks its file's import/use/include statements to build an
//! `ImportsMap` mapping short type names to their origin. When emitting a
//! function's `parameter_types`, the parser looks up each type's innermost
//! identifier in that map to set `TypeOrigin`.

use std::collections::HashMap;

use cha_core::{TypeOrigin, TypeRef};

/// Short type name -> resolved origin. Built once per file.
pub type ImportsMap = HashMap<String, TypeOrigin>;

/// Build a TypeRef and look up its origin in the file's imports map.
/// Falls back to Unknown if not resolved.
pub fn resolve(raw: impl Into<String>, imports: &ImportsMap) -> TypeRef {
    let raw = raw.into();
    let name = strip_decor(&raw);
    let origin = imports.get(&name).cloned().unwrap_or(TypeOrigin::Unknown);
    TypeRef { name, raw, origin }
}

/// Strip reference / pointer / mut decorations and unwrap common container
/// syntaxes to recover the innermost identifier.
///
/// Peels in sequence: `&`, `&mut`, `mut`, `*` (Rust/C pointer), trailing `*`
/// (C pointer), `?` (TS optional), `[]` (array brackets), then recursively
/// unwraps `Vec<T>`, `Option<T>`, `Box<T>`, `Arc<T>`, `Rc<T>`, `List[T]`,
/// `Optional[T]` (Python), `[]T` (Go slice).
pub fn strip_decor(raw: &str) -> String {
    let s = raw.trim();
    // Peel leading Rust reference + lifetime: `&'a mut Foo` → `Foo`.
    let mut s = s.trim_start_matches('&').trim();
    if s.starts_with('\'') {
        // Drop `'lifetime ` up to whitespace.
        if let Some(rest) = s.split_once(char::is_whitespace) {
            s = rest.1.trim();
        }
    }
    let s = s
        .trim_start_matches("mut ")
        .trim_start_matches("dyn ")
        .trim_start_matches("impl ")
        .trim_start_matches('*')
        .trim_end_matches('*')
        .trim_end_matches('?')
        .trim_end_matches("[]")
        .trim_start_matches("[]")
        .trim();
    // Unwrap one generic container layer at a time.
    for wrapper in ["Vec", "Option", "Box", "Arc", "Rc", "Cell", "RefCell"] {
        if let Some(inner) = peel_rust_generic(s, wrapper) {
            return strip_decor(inner);
        }
    }
    for wrapper in ["List", "Optional", "Set", "Dict", "Iterable"] {
        if let Some(inner) = peel_py_generic(s, wrapper) {
            return strip_decor(inner);
        }
    }
    // Rust path `foo::bar::Baz` — take last segment.
    if let Some(last) = s.rsplit("::").next().filter(|p| !p.is_empty() && *p != s) {
        return last.to_string();
    }
    // Python/Go dotted path `foo.bar.Baz`.
    if let Some(last) = s.rsplit('.').next().filter(|p| !p.is_empty() && *p != s) {
        return last.to_string();
    }
    s.to_string()
}

fn peel_rust_generic<'a>(s: &'a str, wrapper: &str) -> Option<&'a str> {
    let prefix = format!("{wrapper}<");
    let rest = s.strip_prefix(&prefix)?.strip_suffix('>')?;
    Some(rest)
}

fn peel_py_generic<'a>(s: &'a str, wrapper: &str) -> Option<&'a str> {
    let prefix = format!("{wrapper}[");
    let rest = s.strip_prefix(&prefix)?.strip_suffix(']')?;
    Some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_decor_primitives() {
        assert_eq!(strip_decor("i32"), "i32");
        assert_eq!(strip_decor("bool"), "bool");
    }

    #[test]
    fn strip_decor_rust_refs() {
        assert_eq!(strip_decor("&Node"), "Node");
        assert_eq!(strip_decor("&mut Vec<Finding>"), "Finding");
        assert_eq!(strip_decor("&'a String"), "String");
    }

    #[test]
    fn strip_decor_rust_paths() {
        assert_eq!(strip_decor("tree_sitter::Node"), "Node");
        assert_eq!(strip_decor("std::collections::HashMap"), "HashMap");
    }

    #[test]
    fn strip_decor_rust_generics() {
        assert_eq!(strip_decor("Option<String>"), "String");
        assert_eq!(strip_decor("Vec<Box<MyTrait>>"), "MyTrait");
        assert_eq!(strip_decor("Arc<RefCell<State>>"), "State");
    }

    #[test]
    fn strip_decor_c_pointers() {
        assert_eq!(strip_decor("char *"), "char");
        assert_eq!(strip_decor("cmark_node_t *"), "cmark_node_t");
    }

    #[test]
    fn strip_decor_ts_and_py_containers() {
        assert_eq!(strip_decor("Foo[]"), "Foo");
        assert_eq!(strip_decor("Foo?"), "Foo");
        assert_eq!(strip_decor("List[int]"), "int");
        assert_eq!(strip_decor("Optional[MyType]"), "MyType");
    }

    #[test]
    fn strip_decor_dotted_paths() {
        assert_eq!(strip_decor("package.module.Type"), "Type");
    }

    #[test]
    fn resolve_uses_imports_map() {
        let mut imports = ImportsMap::new();
        imports.insert("Node".into(), TypeOrigin::External("tree_sitter".into()));
        let tr = resolve("&tree_sitter::Node", &imports);
        assert_eq!(tr.name, "Node");
        assert_eq!(tr.origin, TypeOrigin::External("tree_sitter".into()));
    }

    #[test]
    fn resolve_falls_back_to_unknown() {
        let imports = ImportsMap::new();
        let tr = resolve("SomeUnknownType", &imports);
        assert_eq!(tr.origin, TypeOrigin::Unknown);
    }
}
