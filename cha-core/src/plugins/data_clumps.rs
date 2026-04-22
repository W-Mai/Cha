use std::collections::HashMap;

use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect groups of parameters that repeatedly appear together across functions.
pub struct DataClumpsAnalyzer {
    pub min_clump_size: usize,
    pub min_occurrences: usize,
}

impl Default for DataClumpsAnalyzer {
    fn default() -> Self {
        Self {
            min_clump_size: 3,
            min_occurrences: 3,
        }
    }
}

impl Plugin for DataClumpsAnalyzer {
    fn name(&self) -> &str {
        "data_clumps"
    }

    fn description(&self) -> &str {
        "Repeated parameter type signatures"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        // Build sorted param-type signatures per function
        let sigs: Vec<_> = ctx
            .model
            .functions
            .iter()
            .filter(|f| f.parameter_types.len() >= self.min_clump_size)
            .map(|f| (f, f.parameter_types.join(",")))
            .collect();

        // Count how many functions share the same type signature
        let mut sig_counts: HashMap<&str, usize> = HashMap::new();
        for (_, sig) in &sigs {
            *sig_counts.entry(sig.as_str()).or_default() += 1;
        }

        sigs.iter()
            .filter(|(_, sig)| {
                sig_counts.get(sig.as_str()).copied().unwrap_or(0) >= self.min_occurrences
            })
            .map(|(f, sig)| Finding {
                smell_name: "data_clumps".into(),
                category: SmellCategory::Bloaters,
                severity: Severity::Hint,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: f.start_line,
                    start_col: f.name_col,
                    end_line: f.start_line,
                    end_col: f.name_end_col,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` shares parameter signature [{}] with {} other functions",
                    f.name,
                    sig,
                    sig_counts[sig.as_str()] - 1
                ),
                suggested_refactorings: vec![
                    "Extract Class".into(),
                    "Introduce Parameter Object".into(),
                ],
                ..Default::default()
            })
            .collect()
    }
}
