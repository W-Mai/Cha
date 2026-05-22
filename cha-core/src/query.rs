//! Host-side tree-sitter query helper.
//!
//! Built-in plugins receive `ctx.tree` and `ctx.ts_language` and can run
//! tree-sitter S-expression queries directly via `run_query`. WASM plugins go
//! through the `tree_query` host import (see [`crate::wasm`]) — both paths
//! ultimately call this helper.
//!
//! Lines in [`QueryMatch`] are 1-based to match `FunctionInfo` /
//! `ClassInfo` / `CommentInfo`. Columns are 0-based byte offsets.

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Tree};

#[derive(Debug, Clone)]
pub struct QueryMatch {
    pub capture_name: String,
    pub node_kind: String,
    pub text: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Outer `Vec` = each pattern match, inner = captures within that match
/// (in capture-list order). Empty on pattern compile error rather than panic
/// — pattern strings can come from external plugins.
pub fn run_query(
    tree: &Tree,
    lang: &Language,
    source: &[u8],
    pattern: &str,
) -> Vec<Vec<QueryMatch>> {
    let query = match tree_sitter::Query::new(lang, pattern) {
        Ok(q) => q,
        Err(_) => return vec![],
    };
    let capture_names: Vec<&str> = query.capture_names().to_vec();

    let mut cursor = tree_sitter::QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source);
    let mut results = vec![];
    while let Some(m) = StreamingIterator::next(&mut matches) {
        let captures: Vec<QueryMatch> = m
            .captures
            .iter()
            .map(|c| {
                let name: &str = capture_names.get(c.index as usize).copied().unwrap_or("");
                node_to_match(&c.node, source, name)
            })
            .collect();
        results.push(captures);
    }
    results
}

pub fn run_queries(
    tree: &Tree,
    lang: &Language,
    source: &[u8],
    patterns: &[&str],
) -> Vec<Vec<Vec<QueryMatch>>> {
    patterns
        .iter()
        .map(|p| run_query(tree, lang, source, p))
        .collect()
}

/// 1-based lines (matches `FunctionInfo` convention).
pub fn node_to_match(node: &tree_sitter::Node, source: &[u8], capture_name: &str) -> QueryMatch {
    let text = node.utf8_text(source).unwrap_or("").to_string();
    QueryMatch {
        capture_name: capture_name.to_string(),
        node_kind: node.kind().to_string(),
        text,
        start_line: (node.start_position().row as u32) + 1,
        start_col: node.start_position().column as u32,
        end_line: (node.end_position().row as u32) + 1,
        end_col: node.end_position().column as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_rust(src: &str) -> (Tree, Language) {
        let lang: Language = tree_sitter_rust::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse(src, None).unwrap();
        (tree, lang)
    }

    #[test]
    fn finds_unsafe_blocks() {
        let src = "fn main() { unsafe { let _ = 1; } }";
        let (tree, lang) = parse_rust(src);
        let matches = run_query(&tree, &lang, src.as_bytes(), "(unsafe_block) @b");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0][0].node_kind, "unsafe_block");
        assert_eq!(matches[0][0].start_line, 1);
    }

    #[test]
    fn empty_for_invalid_pattern() {
        let src = "fn main() {}";
        let (tree, lang) = parse_rust(src);
        let matches = run_query(&tree, &lang, src.as_bytes(), "(no_such_node_kind) @x");
        assert!(matches.is_empty());
    }

    #[test]
    fn captures_are_1_based() {
        let src = "// line 1\nfn foo() {}\n";
        let (tree, lang) = parse_rust(src);
        let matches = run_query(&tree, &lang, src.as_bytes(), "(function_item) @f");
        assert_eq!(matches[0][0].start_line, 2);
    }
}
