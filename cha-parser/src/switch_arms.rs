//! Shared utilities for extracting `switch` / `match` arm literal
//! values. Each language dispatches its own node traversal to the
//! helpers here for classifying pattern literals consistently.

use cha_core::ArmValue;
use tree_sitter::Node;

/// Classify a literal node (after the language's case/pattern
/// wrapping has been peeled off) into an [`ArmValue`]. Returns
/// `Other` for non-literal patterns — caller still counts them but
/// they never contribute to stringly-typed-dispatch detection.
pub fn classify_literal(node: Node, src: &[u8]) -> ArmValue {
    let text = node.utf8_text(src).unwrap_or("").trim();
    match node.kind() {
        "string_literal"
        | "string"
        | "interpreted_string_literal"
        | "raw_string_literal"
        | "string_fragment" => ArmValue::Str(strip_quotes(text).to_string()),
        "number_literal" | "integer_literal" | "int_literal" | "integer" | "number" => {
            parse_integer(text)
                .map(ArmValue::Int)
                .unwrap_or(ArmValue::Other)
        }
        "char_literal" => parse_char(text)
            .map(ArmValue::Char)
            .unwrap_or(ArmValue::Other),
        _ => ArmValue::Other,
    }
}

/// Walk the entire function body and push every arm value encountered.
/// The predicate tells us which node is a "switch arm" in the language
/// at hand; from there we look at the arm's pattern children and
/// classify the first meaningful literal node.
pub fn walk_arms<F>(body: Node, src: &[u8], out: &mut Vec<ArmValue>, is_arm: &F)
where
    F: Fn(&Node) -> bool,
{
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        if is_arm(&node) {
            out.push(extract_arm_value(node, src));
            continue;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}

/// For a given arm node, find the first literal-looking descendant and
/// classify it. Non-literal arms (Rust enum variants, Python captures,
/// C `default`) return `Other`.
fn extract_arm_value(arm: Node, src: &[u8]) -> ArmValue {
    // Rust / Python wrap the pattern in `match_pattern` / `case_pattern`;
    // C/TS/Go put the literal directly as a child of the arm node.
    let candidate = arm
        .child_by_field_name("pattern")
        .or_else(|| first_named_child(arm, "match_pattern"))
        .or_else(|| first_named_child(arm, "case_pattern"))
        .unwrap_or(arm);
    if let Some(lit) = find_first_literal(candidate) {
        classify_literal(lit, src)
    } else {
        ArmValue::Other
    }
}

fn first_named_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut c = node.walk();
    node.children(&mut c).find(|n| n.kind() == kind)
}

/// Descend into the node looking for the first literal child. Limited
/// depth walk — enough for `match_pattern > string_literal` and the
/// direct-child case used by C/TS/Go.
fn find_first_literal(node: Node) -> Option<Node> {
    const LITERAL_KINDS: &[&str] = &[
        "string_literal",
        "string",
        "interpreted_string_literal",
        "raw_string_literal",
        "number_literal",
        "integer_literal",
        "int_literal",
        "integer",
        "number",
        "char_literal",
    ];
    if LITERAL_KINDS.contains(&node.kind()) {
        return Some(node);
    }
    let mut stack = vec![node];
    let mut depth = 0;
    while let Some(n) = stack.pop() {
        if depth > 4 {
            continue;
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            if LITERAL_KINDS.contains(&child.kind()) {
                return Some(child);
            }
            stack.push(child);
        }
        depth += 1;
    }
    None
}

fn strip_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(s)
}

fn parse_integer(s: &str) -> Option<i64> {
    let s = s.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '_');
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(&hex.replace('_', ""), 16).ok();
    }
    if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return i64::from_str_radix(&oct.replace('_', ""), 8).ok();
    }
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return i64::from_str_radix(&bin.replace('_', ""), 2).ok();
    }
    s.replace('_', "").parse().ok()
}

fn parse_char(s: &str) -> Option<char> {
    let inner = strip_quotes(s);
    let mut chars = inner.chars();
    let first = chars.next()?;
    // Handle single-char escapes: '\n', '\t', '\\', '\''.
    if first == '\\' {
        return match chars.next()? {
            'n' => Some('\n'),
            't' => Some('\t'),
            'r' => Some('\r'),
            '0' => Some('\0'),
            '\\' => Some('\\'),
            '\'' => Some('\''),
            '"' => Some('"'),
            _ => None,
        };
    }
    if chars.next().is_some() {
        // Multi-char (escape sequence unrecognised or multibyte) — give up.
        return None;
    }
    Some(first)
}
