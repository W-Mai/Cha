use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Configurable thresholds for cyclomatic complexity.
pub struct ComplexityAnalyzer {
    pub warn_threshold: usize,
    pub error_threshold: usize,
}

impl Default for ComplexityAnalyzer {
    fn default() -> Self {
        Self {
            warn_threshold: 10,
            error_threshold: 20,
        }
    }
}

impl Plugin for ComplexityAnalyzer {
    fn name(&self) -> &str {
        "complexity"
    }

    fn description(&self) -> &str {
        "Cyclomatic complexity exceeds threshold"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter_map(|f| self.check_function(ctx, f))
            .collect()
    }
}

impl ComplexityAnalyzer {
    /// Build a finding for a single function if its complexity exceeds the threshold.
    fn check_function(&self, ctx: &AnalysisContext, f: &crate::FunctionInfo) -> Option<Finding> {
        if f.complexity < self.warn_threshold {
            return None;
        }
        Some(Finding {
            smell_name: "high_complexity".into(),
            category: SmellCategory::Bloaters,
            severity: if f.complexity >= self.error_threshold {
                Severity::Error
            } else {
                Severity::Warning
            },
            location: Location {
                path: ctx.file.path.clone(),
                start_line: f.start_line,
                end_line: f.end_line,
                name: Some(f.name.clone()),
                ..Default::default()
            },
            message: format!(
                "Function `{}` has complexity {} (threshold: {})",
                f.name, f.complexity, self.warn_threshold
            ),
            suggested_refactorings: vec![
                "Extract Method".into(),
                "Replace Conditional with Polymorphism".into(),
            ],
            actual_value: Some(f.complexity as f64),
            threshold: Some(self.warn_threshold as f64),
        })
    }
}
