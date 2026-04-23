use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect deep method chain calls (e.g. a.b().c().d()).
pub struct MessageChainAnalyzer {
    pub max_depth: usize,
}

impl Default for MessageChainAnalyzer {
    fn default() -> Self {
        Self { max_depth: 3 }
    }
}

impl Plugin for MessageChainAnalyzer {
    fn name(&self) -> &str {
        "message_chain"
    }

    fn smells(&self) -> Vec<String> {
        vec!["message_chain".into()]
    }

    fn description(&self) -> &str {
        "Deep field access chains (a.b.c.d)"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter(|f| f.chain_depth > self.max_depth)
            .map(|f| Finding {
                smell_name: "message_chain".into(),
                category: SmellCategory::Couplers,
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
                    "Function `{}` has chain depth {} (threshold: {})",
                    f.name, f.chain_depth, self.max_depth
                ),
                suggested_refactorings: vec!["Hide Delegate".into()],
                actual_value: Some(f.chain_depth as f64),
                threshold: Some(self.max_depth as f64),
            })
            .collect()
    }
}
