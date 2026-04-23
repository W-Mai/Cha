use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect Hub-like modules with excessive fan-out (too many imports).
///
/// A module that imports too many other modules acts as a "hub" in the
/// dependency graph, coupling itself to a large portion of the system.
///
/// ## References
///
/// [1] F. Arcelli Fontana, I. Pigazzini, R. Roveda, and M. Zanoni,
///     "Architectural Smells Detected by Tools: a Catalogue Proposal,"
///     in Proc. 13th European Conf. Software Architecture (ECSA), 2019.
///     doi: 10.1145/3344948.3344982.
///
/// [2] R. C. Martin, "Agile Software Development: Principles, Patterns,
///     and Practices," Prentice Hall, 2003. ISBN: 978-0135974445.
///     Chapter 20: Stable Dependencies Principle.
pub struct HubLikeDependencyAnalyzer {
    pub max_imports: usize,
}

impl Default for HubLikeDependencyAnalyzer {
    fn default() -> Self {
        Self { max_imports: 20 }
    }
}

impl Plugin for HubLikeDependencyAnalyzer {
    fn name(&self) -> &str {
        "hub_like_dependency"
    }

    fn smells(&self) -> Vec<String> {
        vec!["hub_like_dependency".into()]
    }

    fn description(&self) -> &str {
        "Hub-like module with too many imports"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let count = ctx
            .model
            .imports
            .iter()
            .filter(|i| !i.is_module_decl)
            .count();
        if count <= self.max_imports {
            return vec![];
        }
        let first = ctx.model.imports.first().map(|i| i.line).unwrap_or(1);
        let first_col = ctx.model.imports.first().map(|i| i.col).unwrap_or(0);
        let last = ctx.model.imports.last().map(|i| i.line).unwrap_or(1);
        vec![Finding {
            smell_name: "hub_like_dependency".into(),
            category: SmellCategory::Couplers,
            severity: Severity::Warning,
            location: Location {
                path: ctx.file.path.clone(),
                start_line: first,
                start_col: first_col,
                end_line: last,
                name: None,
                ..Default::default()
            },
            message: format!(
                "File has {} imports (threshold: {}), acting as a hub — consider splitting",
                count, self.max_imports
            ),
            suggested_refactorings: vec!["Extract Module".into(), "Facade Pattern".into()],
            actual_value: Some(count as f64),
            threshold: Some(self.max_imports as f64),
        }]
    }
}
