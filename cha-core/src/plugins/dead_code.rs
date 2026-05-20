use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect non-exported functions/classes that may be dead code.
/// Note: single-file heuristic — flags unexported items as potential dead code.
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
        // C/C++ files using token-concatenation macros (`#define X(n) foo_##n`)
        // create function references invisible to text/AST search. Skip the file
        // entirely rather than report false positives — common pattern in SVG
        // parsers, dispatch tables, X-macros.
        if matches!(ctx.model.language.as_str(), "c" | "cpp")
            && has_token_concat_macros(&ctx.file.content)
        {
            return vec![];
        }

        let mut findings = Vec::new();
        check_dead_functions(ctx, &mut findings);
        check_dead_classes(ctx, &mut findings);
        findings
    }
}

/// Detect `#define ... ##` patterns indicating token-concatenation macros.
/// These hide function references from text search. Conservative: scan for
/// `#define` lines containing `##` (cheap, no false negatives for the pattern).
fn has_token_concat_macros(content: &str) -> bool {
    let mut in_define = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#define") {
            in_define = true;
        }
        if in_define && trimmed.contains("##") {
            return true;
        }
        // Multi-line #define ends on a line not ending with backslash continuation
        if in_define && !line.trim_end().ends_with('\\') {
            in_define = false;
        }
    }
    false
}

/// Flag unexported, unreferenced functions as potential dead code.
fn check_dead_functions(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    for f in &ctx.model.functions {
        if f.is_exported || is_entry_point(&f.name) {
            continue;
        }
        if !is_referenced(&ctx.file.content, &f.name, f.start_line, f.end_line) {
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
}

/// Flag unexported, unreferenced classes as potential dead code.
fn check_dead_classes(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    for c in &ctx.model.classes {
        if c.is_exported || is_referenced(&ctx.file.content, &c.name, c.start_line, c.end_line) {
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

/// Check if a name is referenced outside its own definition lines.
fn is_referenced(content: &str, name: &str, def_start: usize, def_end: usize) -> bool {
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
    fn macro_detection_finds_simple_concat() {
        let src = "#define FN(name) handle_##name##_attr\n";
        assert!(has_token_concat_macros(src));
    }

    #[test]
    fn macro_detection_finds_multiline_concat() {
        let src = "#define X(a, b) \\\n    foo_##a##_##b\n";
        assert!(has_token_concat_macros(src));
    }

    #[test]
    fn macro_detection_ignores_concat_outside_define() {
        // `##` outside a #define line — e.g. in a comment or string — should not trigger.
        let src = "// this comment has ## in it\nlet s = \"a##b\";\n";
        assert!(!has_token_concat_macros(src));
    }

    #[test]
    fn macro_detection_ignores_define_without_concat() {
        let src = "#define MAX(a, b) ((a) > (b) ? (a) : (b))\n";
        assert!(!has_token_concat_macros(src));
    }

    #[test]
    fn macro_detection_handles_no_macros() {
        let src = "int main() { return 0; }\n";
        assert!(!has_token_concat_macros(src));
    }
}
