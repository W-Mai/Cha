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
/// Falls back to `is_universal_primitive` (common Rust/Python/JS primitives
/// and prelude types that rarely show up in import statements) before
/// giving up as Unknown.
pub fn resolve(raw: impl Into<String>, imports: &ImportsMap) -> TypeRef {
    let raw = raw.into();
    let name = strip_decor(&raw);
    let origin = imports
        .get(&name)
        .cloned()
        .unwrap_or_else(|| fallback_origin(&name));
    TypeRef { name, raw, origin }
}

fn fallback_origin(name: &str) -> TypeOrigin {
    if is_universal_primitive(name) {
        TypeOrigin::Primitive
    } else {
        TypeOrigin::Unknown
    }
}

/// Primitives / prelude-visible types used across languages without an
/// explicit import. Keeps the detector from firing on ubiquitous types.
fn is_universal_primitive(name: &str) -> bool {
    UNIVERSAL_PRIMITIVES.contains(&name)
}

const UNIVERSAL_PRIMITIVES: &[&str] = &[
    // Rust primitives + prelude
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "f32",
    "f64",
    "bool",
    "char",
    "str",
    "String",
    "Vec",
    "Option",
    "Result",
    "Box",
    "Arc",
    "Rc",
    "Cell",
    "RefCell",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "Path",
    "PathBuf",
    "OsStr",
    "OsString",
    // Python built-ins (also via `typing`, but Python files often omit them)
    "int",
    "float",
    "bytes",
    "bytearray",
    "list",
    "dict",
    "set",
    "tuple",
    "None",
    "Any",
    // TypeScript / JS built-ins
    "number",
    "string",
    "boolean",
    "null",
    "undefined",
    "void",
    "never",
    "unknown",
    "any",
    "Array",
    "Promise",
    "Map",
    "Set",
    "Date",
    "Error",
    "RegExp",
    "Function",
    "Object",
    // Go built-ins already handled in golang.rs resolver
    // C/C++ handled in c_imports.rs
];

/// Strip reference / pointer / mut decorations and unwrap common container
/// syntaxes to recover the innermost identifier.
///
/// Peels in sequence: `&`, `&mut`, `mut`, `*` (Rust/C pointer), trailing `*`
/// (C pointer), `?` (TS optional), `[]` (array brackets), then recursively
/// unwraps `Vec<T>`, `Option<T>`, `Box<T>`, `Arc<T>`, `Rc<T>`, `List[T]`,
/// `Optional[T]` (Python), `[]T` (Go slice).
pub fn strip_decor(raw: &str) -> String {
    let s = strip_rust_refs_and_slice(raw.trim());
    if let Some(recur) = rust_slice_inner(s) {
        return strip_decor(recur);
    }
    let s = strip_basic_decor(s);
    if let Some(inner) = peel_any_container(s) {
        return strip_decor(inner);
    }
    take_last_segment(s)
}

/// Peel the leading `&`/`&'lifetime ` reference from a Rust type. Returns the
/// slice after the reference (or the original string if there was none).
fn strip_rust_refs_and_slice(s: &str) -> &str {
    let s = s.trim_start_matches('&').trim();
    if !s.starts_with('\'') {
        return s;
    }
    s.split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .unwrap_or(s)
}

fn rust_slice_inner(s: &str) -> Option<&str> {
    s.strip_prefix('[').and_then(|r| r.strip_suffix(']'))
}

fn strip_basic_decor(s: &str) -> &str {
    s.trim_start_matches("mut ")
        .trim_start_matches("dyn ")
        .trim_start_matches("impl ")
        .trim_start_matches('*')
        .trim_end_matches('*')
        .trim_end_matches('?')
        .trim_end_matches("[]")
        .trim_start_matches("[]")
        .trim()
}

fn peel_any_container(s: &str) -> Option<&str> {
    for wrapper in ["Vec", "Option", "Box", "Arc", "Rc", "Cell", "RefCell"] {
        if let Some(inner) = peel_rust_generic(s, wrapper) {
            return Some(inner);
        }
    }
    for wrapper in ["List", "Optional", "Set", "Dict", "Iterable"] {
        if let Some(inner) = peel_py_generic(s, wrapper) {
            return Some(inner);
        }
    }
    None
}

fn take_last_segment(s: &str) -> String {
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
    fn strip_decor_rust_slice() {
        assert_eq!(strip_decor("[PathBuf]"), "PathBuf");
        assert_eq!(strip_decor("&[Finding]"), "Finding");
        assert_eq!(strip_decor("&[&DetailClass]"), "DetailClass");
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
