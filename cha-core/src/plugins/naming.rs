use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Check naming conventions for functions and classes.
pub struct NamingAnalyzer {
    pub min_name_length: usize,
    pub max_name_length: usize,
}

impl Default for NamingAnalyzer {
    fn default() -> Self {
        Self {
            min_name_length: 2,
            max_name_length: 50,
        }
    }
}

impl Plugin for NamingAnalyzer {
    fn name(&self) -> &str {
        "naming"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        for f in &ctx.model.functions {
            if let Some(finding) = check_name(
                &f.name,
                "Function",
                &ctx.file.path,
                f.start_line,
                f.end_line,
                self.min_name_length,
                self.max_name_length,
            ) {
                findings.push(finding);
            }
        }

        for c in &ctx.model.classes {
            // Classes should be PascalCase
            if !c.name.is_empty() && c.name.chars().next().is_some_and(|ch| ch.is_lowercase()) {
                findings.push(Finding {
                    smell_name: "naming_convention".into(),
                    category: SmellCategory::Bloaters,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: c.start_line,
                        end_line: c.end_line,
                        name: Some(c.name.clone()),
                    },
                    message: format!("Class `{}` should use PascalCase", c.name),
                    suggested_refactorings: vec!["Rename Method".into()],
                });
            }

            if let Some(finding) = check_name(
                &c.name,
                "Class",
                &ctx.file.path,
                c.start_line,
                c.end_line,
                self.min_name_length,
                self.max_name_length,
            ) {
                findings.push(finding);
            }
        }

        findings
    }
}

fn check_name(
    name: &str,
    kind: &str,
    path: &std::path::Path,
    start_line: usize,
    end_line: usize,
    min_len: usize,
    max_len: usize,
) -> Option<Finding> {
    if name.len() < min_len {
        Some(Finding {
            smell_name: "naming_too_short".into(),
            category: SmellCategory::Bloaters,
            severity: Severity::Warning,
            location: Location {
                path: path.to_path_buf(),
                start_line,
                end_line,
                name: Some(name.to_string()),
            },
            message: format!(
                "{} `{}` name is too short ({} chars, min: {})",
                kind,
                name,
                name.len(),
                min_len
            ),
            suggested_refactorings: vec!["Rename Method".into()],
        })
    } else if name.len() > max_len {
        Some(Finding {
            smell_name: "naming_too_long".into(),
            category: SmellCategory::Bloaters,
            severity: Severity::Hint,
            location: Location {
                path: path.to_path_buf(),
                start_line,
                end_line,
                name: Some(name.to_string()),
            },
            message: format!(
                "{} `{}` name is too long ({} chars, max: {})",
                kind,
                name,
                name.len(),
                max_len
            ),
            suggested_refactorings: vec!["Rename Method".into()],
        })
    } else {
        None
    }
}
