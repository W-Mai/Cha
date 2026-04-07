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
        self.check_functions(ctx, &mut findings);
        self.check_classes(ctx, &mut findings);
        findings
    }
}

impl NamingAnalyzer {
    fn check_functions(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
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
    }

    fn check_classes(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for c in &ctx.model.classes {
            if let Some(f) = check_pascal_case(c, &ctx.file.path) {
                findings.push(f);
            }
            if let Some(f) = check_name(
                &c.name,
                "Class",
                &ctx.file.path,
                c.start_line,
                c.end_line,
                self.min_name_length,
                self.max_name_length,
            ) {
                findings.push(f);
            }
        }
    }
}

/// Check if a class name violates PascalCase convention.
fn check_pascal_case(c: &crate::ClassInfo, path: &std::path::Path) -> Option<Finding> {
    if c.name.is_empty() || c.name.chars().next().is_some_and(|ch| ch.is_uppercase()) {
        return None;
    }
    Some(Finding {
        smell_name: "naming_convention".into(),
        category: SmellCategory::Bloaters,
        severity: Severity::Hint,
        location: Location {
            path: path.to_path_buf(),
            start_line: c.start_line,
            end_line: c.end_line,
            name: Some(c.name.clone()),
        },
        message: format!("Class `{}` should use PascalCase", c.name),
        suggested_refactorings: vec!["Rename Method".into()],
    })
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
    let (smell, severity, qualifier, limit) = if name.len() < min_len {
        ("naming_too_short", Severity::Warning, "short", min_len)
    } else if name.len() > max_len {
        ("naming_too_long", Severity::Hint, "long", max_len)
    } else {
        return None;
    };
    let bound_label = if qualifier == "short" { "min" } else { "max" };
    Some(Finding {
        smell_name: smell.into(),
        category: SmellCategory::Bloaters,
        severity,
        location: Location {
            path: path.to_path_buf(),
            start_line,
            end_line,
            name: Some(name.to_string()),
        },
        message: format!(
            "{} `{}` name is too {} ({} chars, {}: {})",
            kind,
            name,
            qualifier,
            name.len(),
            bound_label,
            limit
        ),
        suggested_refactorings: vec!["Rename Method".into()],
    })
}
