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

        // Collect all names referenced in the file content
        let content = &ctx.file.content;

        for f in &ctx.model.functions {
            if !f.is_exported && !is_referenced(content, &f.name, f.start_line, f.end_line) {
                findings.push(Finding {
                    smell_name: "dead_code".into(),
                    category: SmellCategory::Dispensables,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: f.start_line,
                        end_line: f.end_line,
                        name: Some(f.name.clone()),
                    },
                    message: format!("Function `{}` is not exported and may be unused", f.name),
                    suggested_refactorings: vec!["Remove dead code".into()],
                });
            }
        }

        for c in &ctx.model.classes {
            if !c.is_exported && !is_referenced(content, &c.name, c.start_line, c.end_line) {
                findings.push(Finding {
                    smell_name: "dead_code".into(),
                    category: SmellCategory::Dispensables,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: c.start_line,
                        end_line: c.end_line,
                        name: Some(c.name.clone()),
                    },
                    message: format!("Class `{}` is not exported and may be unused", c.name),
                    suggested_refactorings: vec!["Remove dead code".into()],
                });
            }
        }

        findings
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
