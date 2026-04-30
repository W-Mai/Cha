//! C++-only helpers that live outside `c_lang.rs` to keep that file
//! below the `large_file` threshold. Handles qualified-identifier name
//! resolution (`Foo::bar`, `A::B::c`, `::global`) and attribution of
//! out-of-class method definitions to their owning class.

use cha_core::ClassInfo;
use tree_sitter::Node;

fn text<'a>(n: Node, src: &'a [u8]) -> &'a str {
    n.utf8_text(src).unwrap_or("")
}

/// Walk a C/C++ declarator chain down to its bare identifier name.
/// Handles `int x`, `int *x`, `int x[]`, `int (*x)(...)`, `int &x`.
/// Anonymous or malformed declarators return an empty string.
pub(crate) fn c_param_name(decl: Node, src: &[u8]) -> String {
    let mut cur = decl;
    loop {
        match cur.kind() {
            "identifier" => return text(cur, src).to_string(),
            "pointer_declarator"
            | "array_declarator"
            | "function_declarator"
            | "reference_declarator" => match cur.child_by_field_name("declarator") {
                Some(n) => cur = n,
                None => {
                    let mut c = cur.walk();
                    let Some(next) = cur.children(&mut c).find(|n| n.is_named()) else {
                        return String::new();
                    };
                    cur = next;
                }
            },
            _ => return String::new(),
        }
    }
}

/// For `A::B::c` (nested `qualified_identifier`), return the `c` leaf.
/// Leaf kinds are the identifier forms used for function names.
pub(crate) fn qualified_identifier_leaf(node: Node) -> Option<Node> {
    let mut cur = node;
    while cur.kind() == "qualified_identifier" {
        let next = cur.child_by_field_name("name")?;
        cur = next;
    }
    matches!(
        cur.kind(),
        "identifier" | "field_identifier" | "destructor_name" | "operator_name"
    )
    .then_some(cur)
}

/// Bump `method_count` on the in-file `ClassInfo` whose name matches the
/// last segment of `qualifier` (the class part of `A::B::Foo::bar`).
/// Strips any `<...>` template arguments so a qualifier like
/// `"Foo<int>"` (from a template specialisation) still attributes to
/// `class Foo`. Cross-file attribution happens later in
/// `cha-cli::c_oop_enrich`.
pub(crate) fn attach_to_class(qualifier: &str, classes: &mut [ClassInfo]) {
    let last = qualifier.rsplit("::").next().unwrap_or(qualifier);
    let bare = strip_template_args(last);
    if let Some(c) = classes.iter_mut().find(|c| c.name == bare) {
        c.method_count += 1;
        c.has_behavior = true;
    }
}

/// Drop a trailing `<...>` from an identifier. Handles nested angle
/// brackets by counting depth rather than greedy matching. `"Foo<int>"`
/// → `"Foo"`, `"Map<K, V<T>>"` → `"Map"`, `"plain"` → `"plain"`.
fn strip_template_args(s: &str) -> &str {
    let Some(lt) = s.find('<') else {
        return s;
    };
    &s[..lt]
}

/// Resolve a class_specifier's `name` field into (name, name_col, name_end_col).
/// Unwraps `template_type` (e.g. `Foo<T>`) down to its underlying
/// `type_identifier` so the stored class name is the bare `Foo`, not
/// `Foo<T>` — keeps class-home lookups consistent between template
/// decl sites and specialisations.
pub(crate) fn class_name_triple(name_node: Option<Node>, src: &[u8]) -> (String, usize, usize) {
    let Some(n) = name_node else {
        return (String::new(), 0, 0);
    };
    let name_n = if n.kind() == "template_type" {
        n.child_by_field_name("name").unwrap_or(n)
    } else {
        n
    };
    let name = text(name_n, src).to_string();
    (
        name,
        name_n.start_position().column,
        name_n.end_position().column,
    )
}

/// Extract the primary base class from a C++ `class_specifier` or
/// `struct_specifier` — the first `type_identifier` under the
/// `base_class_clause` child. Handles `class Foo : public Base`,
/// `struct Foo : Base`, `class Foo : public Base<T>` (base inside
/// `template_type`), and multi-base (only the first is returned, same
/// as a single-parent inheritance model).
///
/// Returns `None` if there is no `base_class_clause` (plain C struct or
/// class with no bases), so callers can fall back to whatever heuristic
/// they want (the existing code falls back to first-field type).
pub(crate) fn extract_cpp_base(class_node: Node, src: &[u8]) -> Option<String> {
    let clause = find_child(class_node, "base_class_clause")?;
    let base = find_first_type_in_clause(clause)?;
    let raw = text(base, src);
    Some(strip_template_args(raw).to_string())
}

fn find_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut c = node.walk();
    node.children(&mut c).find(|n| n.kind() == kind)
}

fn find_first_type_in_clause(clause: Node) -> Option<Node> {
    // Direct children of base_class_clause: `:`, access_specifier,
    // type_identifier, template_type, `,`, access_specifier, type_identifier …
    // Grab the first `type_identifier` or the `type_identifier` inside
    // the first `template_type`.
    let mut cursor = clause.walk();
    for ch in clause.children(&mut cursor) {
        match ch.kind() {
            "type_identifier" => return Some(ch),
            "template_type" => return ch.child_by_field_name("name").or(Some(ch)),
            _ => {}
        }
    }
    None
}

/// Return the class/namespace prefix of a function definition's declarator,
/// e.g. `"Foo"` for `void Foo::bar()`, `"A::B"` for `A::B::c`, `None` for
/// plain `bar` or global-scope `::bar`.
pub(crate) fn extract_class_qualifier(node: Node, src: &[u8]) -> Option<String> {
    let declarator = node.child_by_field_name("declarator")?;
    let qid = descend_to_qualified_identifier(declarator)?;
    build_qualifier_chain(qid, src)
}

fn descend_to_qualified_identifier(declarator: Node) -> Option<Node> {
    let mut cur = declarator;
    loop {
        if cur.kind() == "qualified_identifier" {
            return Some(cur);
        }
        // `reference_declarator` lacks a named `declarator` field — its
        // sub-declarator is an unnamed positional child. Fall back to the
        // first named descendant when the field isn't present.
        cur = match cur.child_by_field_name("declarator") {
            Some(n) => n,
            None => first_named_child(cur)?,
        };
    }
}

fn first_named_child(node: Node) -> Option<Node> {
    let mut c = node.walk();
    node.children(&mut c).find(|n| n.is_named())
}

fn build_qualifier_chain(qid: Node, src: &[u8]) -> Option<String> {
    // Collect every `scope` down the qualified_identifier spine. When the
    // outermost node has no `scope` (`::foo` at translation-unit scope)
    // there's nothing to attribute to.
    let mut parts = Vec::new();
    let mut cur = qid;
    while cur.kind() == "qualified_identifier" {
        if let Some(scope) = cur.child_by_field_name("scope") {
            parts.push(text(scope, src).to_string());
        }
        match cur.child_by_field_name("name") {
            Some(next) => cur = next,
            None => break,
        }
    }
    (!parts.is_empty()).then(|| parts.join("::"))
}
