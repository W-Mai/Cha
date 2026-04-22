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

        // Skip Rust mod declarations — module organization, not coupling
        // Skip module declarations — module organization, not coupling
        let import_count = ctx
            .model
            .imports
            .iter()
            .filter(|i| !i.is_module_decl)
            .count();

        let first = ctx.model.imports.first().map(|i| i.line).unwrap_or(1);
        let last = ctx.model.imports.last().map(|i| i.line).unwrap_or(1);

        if import_count > self.max_imports {
            findings.push(Finding {
                smell_name: "high_coupling".into(),
                category: SmellCategory::Couplers,
                severity: if import_count > self.max_imports * 2 {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: first,
                    end_line: last,
                    name: None,
                },
                message: format!(
                    "File has {} imports (threshold: {})",
                    import_count, self.max_imports
                ),
                suggested_refactorings: vec!["Move Method".into(), "Extract Class".into()],
                actual_value: Some(import_count as f64),
                threshold: Some(self.max_imports as f64),
            });
        }

        findings
    }
}
