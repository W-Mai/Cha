use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect classes where most methods only delegate to another object.
pub struct MiddleManAnalyzer {
    pub min_methods: usize,
    pub delegation_ratio: f64,
}

impl Default for MiddleManAnalyzer {
    fn default() -> Self {
        Self {
            min_methods: 3,
            delegation_ratio: 0.5,
        }
    }
}

impl Plugin for MiddleManAnalyzer {
    fn name(&self) -> &str {
        "middle_man"
    }

    fn smells(&self) -> Vec<String> {
        vec!["middle_man".into()]
    }

    fn description(&self) -> &str {
        "Class that only delegates to others"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .classes
            .iter()
            .filter(|c| {
                c.method_count >= self.min_methods
                    && c.delegating_method_count > 0
                    && (c.delegating_method_count as f64 / c.method_count as f64)
                        >= self.delegation_ratio
            })
            .map(|c| Finding {
                smell_name: "middle_man".into(),
                category: SmellCategory::Couplers,
                severity: Severity::Hint,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: c.start_line,
                    start_col: c.name_col,
                    end_line: c.start_line,
                    end_col: c.name_end_col,
                    name: Some(c.name.clone()),
                },
                message: format!(
                    "Class `{}` delegates {}/{} methods, acting as a middle man",
                    c.name, c.delegating_method_count, c.method_count
                ),
                suggested_refactorings: vec!["Remove Middle Man".into()],
                actual_value: Some(c.delegating_method_count as f64 / c.method_count as f64),
                threshold: Some(self.delegation_ratio),
            })
            .collect()
    }
}
