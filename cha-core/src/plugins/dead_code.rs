use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect non-exported functions/classes that may be dead code.
/// Note: single-file heuristic — flags unexported items as potential dead code.
pub struct DeadCodeAnalyzer;

impl Plugin for DeadCodeAnalyzer {
    fn name(&self) -> &str {
        "dead_code"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        check_dead_functions(ctx, &mut findings);
        check_dead_classes(ctx, &mut findings);
        findings
    }
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
                f.end_line,
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
            c.end_line,
            &c.name,
            "Class",
        ));
    }
}

/// Build a dead code finding for a given symbol.
fn make_dead_code_finding(
    ctx: &AnalysisContext,
    start_line: usize,
    end_line: usize,
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
            end_line,
            name: Some(name.to_string()),
        },
        message: format!("{} `{}` is not exported and may be unused", kind, name),
        suggested_refactorings: vec!["Remove dead code".into()],
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
