//! C++-only helpers that live outside `c_lang.rs` to keep that file
//! below the `large_file` threshold. Handles qualified-identifier name
//! resolution (`Foo::bar`, `A::B::c`, `::global`) and attribution of
//! out-of-class method definitions to their owning class.

use cha_core::ClassInfo;
use tree_sitter::Node;

fn text<'a>(n: Node, src: &'a [u8]) -> &'a str {
    n.utf8_text(src).unwrap_or("")
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
/// Cross-file attribution happens later in `cha-cli::c_oop_enrich`.
pub(crate) fn attach_to_class(qualifier: &str, classes: &mut [ClassInfo]) {
    let last = qualifier.rsplit("::").next().unwrap_or(qualifier);
    if let Some(c) = classes.iter_mut().find(|c| c.name == last) {
        c.method_count += 1;
        c.has_behavior = true;
    }
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
