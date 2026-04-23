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

    fn smells(&self) -> Vec<String> {
        vec!["switch_statement".into()]
    }

    fn description(&self) -> &str {
        "Excessive switch/match arms"
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
                    start_col: f.name_col,
                    end_line: f.start_line,
                    end_col: f.name_end_col,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` has {} switch/match arms (threshold: {})",
                    f.name, f.switch_arms, self.max_arms
                ),
                suggested_refactorings: vec!["Replace Conditional with Polymorphism".into()],
                actual_value: Some(f.switch_arms as f64),
                threshold: Some(self.max_arms as f64),
            })
            .collect()
    }
}
