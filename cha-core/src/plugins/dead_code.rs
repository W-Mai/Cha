use std::collections::HashSet;

use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect non-exported functions/classes that may be dead code.
///
/// Three signals stack:
/// - `is_in_file_referenced` — same-file usage via AST identifier scan
///   (precise; substring matches in strings / comments don't count).
/// - `ctx.project.is_called_externally` — cross-file call graph from parser.
/// - `collect_token_concat_targets` — for C/C++ files, scan `#define ... ##`
///   macros and per-call-site invocations to recover potential function names
///   that the macro would expand to (e.g. `_handle##X##Attr` paired with
///   `STYLE_DEF(color, Color, ...)` produces a plausible `_handleColorAttr`).
///   These names are added to the in-file reference set so X-macro dispatch
///   tables don't drown the file in false positives. Imperfect but vastly
///   better than the previous "any `#define ##` skips the whole file" nuke.
///
/// When `ctx.tree` is unavailable, falls back to the legacy substring scan.
pub struct DeadCodeAnalyzer;

impl Plugin for DeadCodeAnalyzer {
    fn name(&self) -> &str {
        "dead_code"
    }

    fn smells(&self) -> Vec<String> {
        vec!["dead_code".into()]
    }

    fn description(&self) -> &str {
        "Unexported and unreferenced code"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let positions = build_identifier_positions(ctx);
        let mut findings = Vec::new();
        check_dead_functions(ctx, &positions, &mut findings);
        check_dead_classes(ctx, &positions, &mut findings);
        findings
    }
}

/// Build a map: identifier name → list of (line, col) positions where the
/// identifier appears. We use this to check if a symbol is referenced anywhere
/// outside its own definition — substring matches in strings/comments don't
/// count because we walk AST nodes.
///
/// Returns `None` when AST is unavailable so callers fall back to legacy
/// substring scan.
fn build_identifier_positions(ctx: &AnalysisContext) -> Option<IdentifierPositions> {
    let tree = ctx.tree?;
    let lang = ctx.ts_language?;
    let source = ctx.file.content.as_bytes();

    let mut positions: HashSet<(String, u32, u32)> = HashSet::new();
    // Capture all identifier-like nodes regardless of context. The
    // dead-code check then asks: "is `name` mentioned anywhere outside
    // [def_start, def_end]?" — function-pointer assignments, struct
    // initializers, type references, calls all count.
    for pat in [
        "(identifier) @x",
        "(type_identifier) @x",
        "(field_identifier) @x",
        "(property_identifier) @x",
    ] {
        for matches in crate::query::run_query(tree, lang, source, pat) {
            for cap in matches {
                positions.insert((cap.text, cap.start_line, cap.end_col));
            }
        }
    }

    let mut tokens: HashSet<String> = HashSet::new();
    if matches!(ctx.model.language.as_str(), "c" | "cpp") {
        tokens.extend(collect_token_concat_targets(&ctx.file.content));
    }

    Some(IdentifierPositions { positions, tokens })
}

struct IdentifierPositions {
    /// (name, line, end_col) for every identifier node we found.
    positions: HashSet<(String, u32, u32)>,
    /// Names plausibly produced by token-concat macros (C/C++ only).
    tokens: HashSet<String>,
}

impl IdentifierPositions {
    /// True if `name` appears as an identifier anywhere outside the
    /// [def_start, def_end] line range. Token-concat targets count as
    /// referenced regardless of position.
    fn referenced(&self, name: &str, def_start: usize, def_end: usize) -> bool {
        if self.tokens.contains(name) {
            return true;
        }
        for (text, line, _) in &self.positions {
            if text != name {
                continue;
            }
            let l = *line as usize;
            if l < def_start || l > def_end {
                return true;
            }
        }
        false
    }
}

/// Heuristic recovery of names produced by token-paste macros.
///
/// Strategy:
/// 1. Find every `#define NAME(...)` whose body contains `##`.
/// 2. From those bodies, pull tokens of the form `prefix##ARG##suffix`.
/// 3. Then scan the file for `NAME(...)` invocations and try every argument
///    in the call site as the paste candidate (we don't reliably know which
///    parameter the body pasted, especially for body-less macro lookups).
/// 4. Combine prefix + arg + suffix to get plausible expansion names.
///
/// Imperfect — multi-paste, nested macros, and parameter renaming all break
/// this — but it covers the common dispatch-table case (e.g. thorvg's
/// `STYLE_DEF(color, Color, ...)` paired with `_handle##Field##Attr`).
fn collect_token_concat_targets(content: &str) -> HashSet<String> {
    let mut targets = HashSet::new();
    for tmpl in find_concat_define_templates(content) {
        for call_args in find_macro_invocation_args(content, &tmpl.name) {
            for arg in &call_args {
                for (prefix, suffix) in &tmpl.paste_slots {
                    targets.insert(format!("{prefix}{arg}{suffix}"));
                }
            }
        }
    }
    targets
}

struct ConcatTemplate {
    name: String,
    /// (prefix, suffix) for each `prefix##arg##suffix` slot in the body.
    paste_slots: Vec<(String, String)>,
}

// cha:ignore cognitive_complexity
fn find_concat_define_templates(content: &str) -> Vec<ConcatTemplate> {
    let mut out = Vec::new();
    let mut current_define: Option<(String, String)> = None;
    for line in content.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("#define ") {
            // Extract name + parameter list.
            let (name_part, body) = match rest.split_once(')') {
                Some(pair) => pair,
                None => continue,
            };
            let name = name_part
                .split_once('(')
                .map(|(n, _)| n.trim())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            current_define = Some((name, body.to_string()));
        } else if let Some((_, body)) = current_define.as_mut() {
            body.push_str(line);
        }
        let line_continues = line.trim_end().ends_with('\\');
        if !line_continues && let Some((name, body)) = current_define.take() {
            // Done collecting this define. Pull paste slots.
            let slots = extract_paste_slots(&body);
            if !slots.is_empty() {
                out.push(ConcatTemplate {
                    name,
                    paste_slots: slots,
                });
            }
        }
    }
    out
}

/// Pull `prefix##X##suffix` tokens from a body. Returns `(prefix, suffix)`
/// pairs assuming the parameter being pasted is the first macro arg
/// (heuristic — covers `STYLE_DEF(color, Color, ...)` style).
// cha:ignore cognitive_complexity
// cha:ignore high_complexity
fn extract_paste_slots(body: &str) -> Vec<(String, String)> {
    let mut slots = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if &bytes[i..i + 2] == b"##" {
            // Walk back for prefix (identifier-ish characters).
            let mut start = i;
            while start > 0 && is_ident_byte(bytes[start - 1]) {
                start -= 1;
            }
            let prefix = std::str::from_utf8(&bytes[start..i])
                .unwrap_or("")
                .to_string();
            // Skip the parameter name between ## ...
            let mut mid = i + 2;
            while mid < bytes.len() && is_ident_byte(bytes[mid]) {
                mid += 1;
            }
            // Optional trailing ## suffix.
            let mut suffix = String::new();
            if mid + 2 <= bytes.len() && &bytes[mid..mid + 2] == b"##" {
                let mut end = mid + 2;
                while end < bytes.len() && is_ident_byte(bytes[end]) {
                    end += 1;
                }
                suffix = std::str::from_utf8(&bytes[mid + 2..end])
                    .unwrap_or("")
                    .to_string();
                i = end;
            } else {
                i = mid;
            }
            if !prefix.is_empty() || !suffix.is_empty() {
                slots.push((prefix, suffix));
            }
        } else {
            i += 1;
        }
    }
    slots
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Return all simple-identifier arguments of every `macro_name(...)` invocation
/// in `content`. Each invocation produces `Vec<String>` (one per arg position).
/// Args that aren't bare identifiers (literals, complex expressions) are
/// dropped — they wouldn't survive `##` paste anyway.
fn find_macro_invocation_args(content: &str, macro_name: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for line in content.lines() {
        let t = line.trim_start();
        if !t.starts_with(macro_name) {
            continue;
        }
        let rest = &t[macro_name.len()..];
        if !rest.trim_start().starts_with('(') {
            continue;
        }
        let after_paren_pos = rest.find('(').map(|p| p + 1).unwrap_or(0);
        let inside_to_eol = &rest[after_paren_pos..];
        // Stop at the matching `)` if present on this line.
        let inside = inside_to_eol.split(')').next().unwrap_or(inside_to_eol);
        let args: Vec<String> = inside
            .split(',')
            .map(|s| {
                s.trim()
                    .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .to_string()
            })
            .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            .collect();
        if !args.is_empty() {
            out.push(args);
        }
    }
    out
}

/// Flag unexported, unreferenced functions as potential dead code.
fn check_dead_functions(
    ctx: &AnalysisContext,
    positions: &Option<IdentifierPositions>,
    findings: &mut Vec<Finding>,
) {
    for f in &ctx.model.functions {
        if f.is_exported || is_entry_point(&f.name) {
            continue;
        }
        if is_referenced(
            positions,
            &ctx.file.content,
            &f.name,
            f.start_line,
            f.end_line,
        ) {
            continue;
        }
        if let Some(p) = ctx.project
            && p.is_called_externally(&f.name, &ctx.file.path)
        {
            continue;
        }
        findings.push(make_dead_code_finding(
            ctx,
            f.start_line,
            f.name_col,
            f.name_end_col,
            &f.name,
            "Function",
        ));
    }
}

/// Flag unexported, unreferenced classes as potential dead code.
fn check_dead_classes(
    ctx: &AnalysisContext,
    positions: &Option<IdentifierPositions>,
    findings: &mut Vec<Finding>,
) {
    for c in &ctx.model.classes {
        if c.is_exported {
            continue;
        }
        if is_referenced(
            positions,
            &ctx.file.content,
            &c.name,
            c.start_line,
            c.end_line,
        ) {
            continue;
        }
        if let Some(p) = ctx.project
            && p.is_called_externally(&c.name, &ctx.file.path)
        {
            continue;
        }
        findings.push(make_dead_code_finding(
            ctx,
            c.start_line,
            c.name_col,
            c.name_end_col,
            &c.name,
            "Class",
        ));
    }
}

/// Build a dead code finding for a given symbol.
fn make_dead_code_finding(
    ctx: &AnalysisContext,
    start_line: usize,
    name_col: usize,
    name_end_col: usize,
    name: &str,
    kind: &str,
) -> Finding {
    Finding {
        smell_name: "dead_code".into(),
        category: SmellCategory::Dispensables,
        severity: Severity::Hint,
        location: Location {
            path: ctx.file.path.clone(),
            start_line,
            start_col: name_col,
            end_line: start_line,
            end_col: name_end_col,
            name: Some(name.to_string()),
        },
        message: format!("{} `{}` is not exported and may be unused", kind, name),
        suggested_refactorings: vec!["Remove dead code".into()],
        ..Default::default()
    }
}

/// Use the AST-derived identifier positions when available; fall back to
/// substring scan when no tree was attached (legacy unit-test path).
fn is_referenced(
    positions: &Option<IdentifierPositions>,
    content: &str,
    name: &str,
    def_start: usize,
    def_end: usize,
) -> bool {
    match positions {
        Some(idx) => idx.referenced(name, def_start, def_end),
        None => is_in_file_referenced_legacy(content, name, def_start, def_end),
    }
}

/// Pre-AST fallback: substring scan over each line, skipping definition lines.
/// Kept only for the case where ctx.tree is None (e.g. unit tests that build
/// SourceModel by hand).
fn is_in_file_referenced_legacy(
    content: &str,
    name: &str,
    def_start: usize,
    def_end: usize,
) -> bool {
    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;
        if line_num >= def_start && line_num <= def_end {
            continue;
        }
        if line.contains(name) {
            return true;
        }
    }
    false
}

/// Names that are entry points or framework callbacks, not dead code.
fn is_entry_point(name: &str) -> bool {
    matches!(name, "main" | "new" | "default" | "drop" | "fmt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_paste_slots() {
        // `_handle##name##Attr` → prefix `_handle`, suffix `Attr`
        let slots = extract_paste_slots(" _handle##name##Attr ");
        assert_eq!(slots, vec![("_handle".to_string(), "Attr".to_string())]);
    }

    #[test]
    fn extracts_paste_with_only_prefix() {
        // `foo##name` (no trailing paste) → prefix `foo`, suffix ``
        let slots = extract_paste_slots(" foo##name ");
        assert_eq!(slots, vec![("foo".to_string(), "".to_string())]);
    }

    #[test]
    fn finds_all_macro_args() {
        let src = "STYLE_DEF(color, Color, X);\nSTYLE_DEF(fill, Fill, Y);";
        let args = find_macro_invocation_args(src, "STYLE_DEF");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], vec!["color", "Color", "X"]);
        assert_eq!(args[1], vec!["fill", "Fill", "Y"]);
    }

    #[test]
    fn token_concat_recovers_synthetic_targets() {
        let src = "\
#define STYLE_DEF(short, Long) _handle##Long##Attr
STYLE_DEF(color, Color)
STYLE_DEF(fill, Fill)
";
        // Try every arg per invocation — `Color` produces the real name.
        let targets = collect_token_concat_targets(src);
        assert!(targets.contains("_handleColorAttr"));
        assert!(targets.contains("_handleFillAttr"));
    }
}
