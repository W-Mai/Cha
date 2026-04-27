//! Shared helpers for extracting `type X = Y` aliases across languages.
//! Each language's parser has a thin wrapper that walks its own grammar and
//! delegates to functions here — keeps the per-language files below the
//! `large_file` threshold and puts related logic in one place.

use tree_sitter::Node;

fn text<'a>(n: Node, src: &'a [u8]) -> &'a str {
    n.utf8_text(src).unwrap_or("")
}

/// Rust `type_item` — no fields. Alias name is the first `type_identifier`;
/// RHS is the first named child after the `=` token (skipping
/// `type_parameters`).
pub fn rust(node: Node, src: &[u8]) -> Option<(String, String)> {
    let mut c = node.walk();
    let children: Vec<_> = node.children(&mut c).collect();
    let eq = children.iter().position(|n| n.kind() == "=")?;
    let alias = children[..eq]
        .iter()
        .find(|n| n.kind() == "type_identifier")
        .map(|n| text(*n, src))?;
    let rhs = children[eq + 1..]
        .iter()
        .find(|n| n.is_named() && n.kind() != "type_parameters")
        .map(|n| text(*n, src))?;
    pair(alias, rhs)
}

/// TypeScript `type_alias_declaration` — has `name` and `value` fields.
pub fn typescript(node: Node, src: &[u8]) -> Option<(String, String)> {
    let name = node.child_by_field_name("name")?;
    let value = node.child_by_field_name("value")?;
    pair(text(name, src), text(value, src))
}

/// Python 3.12+ `type_alias_statement` — has `left`/`right` fields.
pub fn python_statement(node: Node, src: &[u8]) -> Option<(String, String)> {
    let lhs = node.child_by_field_name("left")?;
    let rhs = node.child_by_field_name("right")?;
    pair(text(lhs, src), text(rhs, src))
}

/// Pre-3.12 `X: TypeAlias = Y` — an `assignment` with an explicit
/// `TypeAlias` annotation. Only the annotated form is treated as an alias;
/// plain `X = Y` is too ambiguous to classify.
pub fn python_assignment(expr_stmt: Node, src: &[u8]) -> Option<(String, String)> {
    let assign = expr_stmt.named_child(0)?;
    if assign.kind() != "assignment" {
        return None;
    }
    let ann = assign.child_by_field_name("type")?;
    if text(ann, src).trim() != "TypeAlias" {
        return None;
    }
    let lhs = assign.child_by_field_name("left")?;
    let rhs = assign.child_by_field_name("right")?;
    pair(text(lhs, src), text(rhs, src))
}

/// Go `type_alias` — has `name` and `type` fields. Excludes the defined-type
/// form `type X Y` (`type_spec`), which the caller handles separately.
pub fn go(node: Node, src: &[u8]) -> Option<(String, String)> {
    let name = node.child_by_field_name("name")?;
    let ty = node.child_by_field_name("type")?;
    pair(text(name, src), text(ty, src))
}

fn pair(alias: &str, rhs: &str) -> Option<(String, String)> {
    let (a, r) = (alias.trim(), rhs.trim());
    (!a.is_empty() && !r.is_empty() && a != r).then(|| (a.to_string(), r.to_string()))
}
