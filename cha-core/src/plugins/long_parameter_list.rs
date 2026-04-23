use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect functions with too many parameters.
pub struct LongParameterListAnalyzer {
    pub max_params: usize,
}

impl Default for LongParameterListAnalyzer {
    fn default() -> Self {
        Self { max_params: 5 }
    }
}

impl Plugin for LongParameterListAnalyzer {
    fn name(&self) -> &str {
        "long_parameter_list"
    }

    fn smells(&self) -> Vec<String> {
        vec!["long_parameter_list".into()]
    }

    fn description(&self) -> &str {
        "Function has too many parameters"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter(|f| f.parameter_count > self.max_params)
            .map(|f| Finding {
                smell_name: "long_parameter_list".into(),
                category: SmellCategory::Bloaters,
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
                    "Function `{}` has {} parameters (threshold: {})",
                    f.name, f.parameter_count, self.max_params
                ),
                suggested_refactorings: vec![
                    "Introduce Parameter Object".into(),
                    "Preserve Whole Object".into(),
                ],
                actual_value: Some(f.parameter_count as f64),
                threshold: Some(self.max_params as f64),
            })
            .collect()
    }
}
