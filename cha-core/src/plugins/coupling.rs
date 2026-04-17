use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect high coupling via excessive imports.
pub struct CouplingAnalyzer {
    pub max_imports: usize,
}

impl Default for CouplingAnalyzer {
    fn default() -> Self {
        Self { max_imports: 15 }
    }
}

impl Plugin for CouplingAnalyzer {
    fn name(&self) -> &str {
        "coupling"
    }

    fn description(&self) -> &str {
        "Too many imports (high coupling)"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        if ctx.model.imports.len() > self.max_imports {
            findings.push(Finding {
                smell_name: "high_coupling".into(),
                category: SmellCategory::Couplers,
                severity: if ctx.model.imports.len() > self.max_imports * 2 {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: 1,
                    end_line: ctx.model.total_lines,
                    name: None,
                },
                message: format!(
                    "File has {} imports (threshold: {})",
                    ctx.model.imports.len(),
                    self.max_imports
                ),
                suggested_refactorings: vec!["Move Method".into(), "Extract Class".into()],
                actual_value: Some(ctx.model.imports.len() as f64),
                threshold: Some(self.max_imports as f64),
            });
        }

        findings
    }
}
