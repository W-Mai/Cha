use crate::{AnalysisContext, Finding, Plugin, Severity, SmellCategory, func_location};

/// Detect functions with high cognitive complexity.
///
/// Cognitive complexity measures how hard code is to *understand*, unlike
/// cyclomatic complexity which measures testability. It penalizes nesting
/// and rewards linear, readable structures like `switch`.
///
/// ## References
///
/// [1] G. A. Campbell, "Cognitive Complexity: A new way of measuring
///     understandability," SonarSource, 2017.
///     https://www.sonarsource.com/resources/white-papers/cognitive-complexity/
pub struct CognitiveComplexityAnalyzer {
    pub threshold: usize,
}

impl Default for CognitiveComplexityAnalyzer {
    fn default() -> Self {
        Self { threshold: 15 }
    }
}

impl Plugin for CognitiveComplexityAnalyzer {
    fn name(&self) -> &str {
        "cognitive_complexity"
    }

    fn smells(&self) -> Vec<String> {
        vec!["cognitive_complexity".into()]
    }

    fn description(&self) -> &str {
        "Cognitive complexity exceeds threshold"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter(|f| f.cognitive_complexity > self.threshold)
            .map(|f| {
                let severity = if f.cognitive_complexity > self.threshold * 2 {
                    Severity::Error
                } else {
                    Severity::Warning
                };
                Finding {
                    smell_name: "cognitive_complexity".into(),
                    category: SmellCategory::Bloaters,
                    severity,
                    location: func_location(&ctx.file.path, f),
                    message: format!(
                        "Function `{}` has cognitive complexity {} (threshold: {})",
                        f.name, f.cognitive_complexity, self.threshold
                    ),
                    suggested_refactorings: vec![
                        "Extract Method".into(),
                        "Replace Nested Conditional with Guard Clauses".into(),
                    ],
                    actual_value: Some(f.cognitive_complexity as f64),
                    threshold: Some(self.threshold as f64),
                }
            })
            .collect()
    }
}
