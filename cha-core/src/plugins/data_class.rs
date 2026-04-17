use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect classes that only have fields and accessor methods (no real behavior).
pub struct DataClassAnalyzer {
    pub min_fields: usize,
}

impl Default for DataClassAnalyzer {
    fn default() -> Self {
        Self { min_fields: 2 }
    }
}

impl Plugin for DataClassAnalyzer {
    fn name(&self) -> &str {
        "data_class"
    }

    fn description(&self) -> &str {
        "Class with only data, no behavior"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .classes
            .iter()
            .filter_map(|c| {
                if c.is_interface || c.field_count < self.min_fields || c.has_behavior {
                    return None;
                }
                Some(Finding {
                    smell_name: "data_class".into(),
                    category: SmellCategory::Dispensables,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: c.start_line,
                        end_line: c.end_line,
                        name: Some(c.name.clone()),
                    },
                    message: format!(
                        "Class `{}` has {} fields but no behavior methods, consider Move Method",
                        c.name, c.field_count
                    ),
                    suggested_refactorings: vec!["Move Method".into(), "Encapsulate Field".into()],
                    ..Default::default()
                })
            })
            .collect()
    }
}
