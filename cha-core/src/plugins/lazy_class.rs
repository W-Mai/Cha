use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect classes with very few methods and lines (nearly empty wrappers).
pub struct LazyClassAnalyzer {
    pub max_methods: usize,
    pub max_lines: usize,
}

impl Default for LazyClassAnalyzer {
    fn default() -> Self {
        Self {
            max_methods: 1,
            max_lines: 10,
        }
    }
}

impl Plugin for LazyClassAnalyzer {
    fn name(&self) -> &str {
        "lazy_class"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .classes
            .iter()
            .filter_map(|c| {
                if c.is_interface
                    || c.method_count > self.max_methods
                    || c.line_count > self.max_lines
                {
                    return None;
                }
                Some(Finding {
                    smell_name: "lazy_class".into(),
                    category: SmellCategory::Dispensables,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: c.start_line,
                        end_line: c.end_line,
                        name: Some(c.name.clone()),
                    },
                    message: format!(
                        "Class `{}` has only {} method(s) and {} lines, consider Inline Class",
                        c.name, c.method_count, c.line_count
                    ),
                    suggested_refactorings: vec!["Inline Class".into()],
                })
            })
            .collect()
    }
}
