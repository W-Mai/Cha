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

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let count = ctx.model.imports.len();
        if count <= self.max_imports {
            return vec![];
        }
        vec![Finding {
            smell_name: "hub_like_dependency".into(),
            category: SmellCategory::Couplers,
            severity: Severity::Warning,
            location: Location {
                path: ctx.file.path.clone(),
                start_line: 1,
                end_line: ctx.model.total_lines,
                name: None,
            },
            message: format!(
                "File has {} imports (threshold: {}), acting as a hub — consider splitting",
                count, self.max_imports
            ),
            suggested_refactorings: vec!["Extract Module".into(), "Facade Pattern".into()],
        }]
    }
}
