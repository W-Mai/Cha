use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect functions with excessive switch/match arms.
pub struct SwitchStatementAnalyzer {
    pub max_arms: usize,
}

impl Default for SwitchStatementAnalyzer {
    fn default() -> Self {
        Self { max_arms: 8 }
    }
}

impl Plugin for SwitchStatementAnalyzer {
    fn name(&self) -> &str {
        "switch_statement"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter(|f| f.switch_arms > self.max_arms)
            .map(|f| Finding {
                smell_name: "switch_statement".into(),
                category: SmellCategory::OoAbusers,
                severity: Severity::Warning,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: f.start_line,
                    end_line: f.end_line,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` has {} switch/match arms (threshold: {})",
                    f.name, f.switch_arms, self.max_arms
                ),
                suggested_refactorings: vec!["Replace Conditional with Polymorphism".into()],
            })
            .collect()
    }
}
