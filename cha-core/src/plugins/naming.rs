use crate::{AnalysisContext, Finding, Location, Patch, Plugin, Severity, SmellCategory, TextEdit};

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

    fn smells(&self) -> Vec<String> {
        vec![
            "naming_convention".into(),
            "naming_too_short".into(),
            "naming_too_long".into(),
        ]
    }

    fn description(&self) -> &str {
        "Naming convention violations"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        self.check_functions(ctx, &mut findings);
        self.check_classes(ctx, &mut findings);
        findings
    }

    fn try_fix(&self, finding: &Finding, ctx: &AnalysisContext) -> Option<Patch> {
        if finding.smell_name != "naming_convention" {
            return None;
        }
        let name = finding.location.name.as_ref()?;
        let new_name = to_pascal_case(name);
        if new_name == *name {
            return None;
        }
        let tree = ctx.tree?;
        let source = ctx.file.content.as_bytes();
        let mut edits = Vec::new();
        collect_identifier_edits(tree.root_node(), source, name, &new_name, &mut edits);
        if edits.is_empty() {
            return None;
        }
        Some(Patch {
            file: ctx.file.path.clone(),
            edits,
        })
    }
}

fn to_pascal_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

fn collect_identifier_edits(
    node: tree_sitter::Node,
    source: &[u8],
    target: &str,
    new_text: &str,
    out: &mut Vec<TextEdit>,
) {
    if matches!(
        node.kind(),
        "identifier" | "type_identifier" | "field_identifier" | "property_identifier"
    ) && let Ok(text) = node.utf8_text(source)
        && text == target
    {
        out.push(TextEdit {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            new_text: new_text.to_string(),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_identifier_edits(child, source, target, new_text, out);
    }
}

impl NamingAnalyzer {
    fn check_functions(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for f in &ctx.model.functions {
            let check = NameCheck {
                name: &f.name,
                kind: "Function",
                path: &ctx.file.path,
                start_line: f.start_line,
                start_col: f.name_col,
                end_line: f.start_line,
                end_col: f.name_end_col,
            };
            if let Some(finding) = check_name(&check, self.min_name_length, self.max_name_length) {
                findings.push(finding);
            }
        }
    }

    fn check_classes(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for c in &ctx.model.classes {
            if let Some(f) = check_pascal_case(c, &ctx.file.path) {
                findings.push(f);
            }
            let check = NameCheck {
                name: &c.name,
                kind: "Class",
                path: &ctx.file.path,
                start_line: c.start_line,
                start_col: c.name_col,
                end_line: c.start_line,
                end_col: c.name_end_col,
            };
            if let Some(f) = check_name(&check, self.min_name_length, self.max_name_length) {
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
            start_col: c.name_col,
            end_line: c.start_line,
            end_col: c.name_end_col,
            name: Some(c.name.clone()),
        },
        message: format!("Class `{}` should use PascalCase", c.name),
        suggested_refactorings: vec!["Rename Method".into()],
        ..Default::default()
    })
}

struct NameCheck<'a> {
    name: &'a str,
    kind: &'a str,
    path: &'a std::path::Path,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

fn check_name(check: &NameCheck, min_len: usize, max_len: usize) -> Option<Finding> {
    let (smell, severity, qualifier, limit) = if check.name.len() < min_len {
        ("naming_too_short", Severity::Warning, "short", min_len)
    } else if check.name.len() > max_len {
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
            path: check.path.to_path_buf(),
            start_line: check.start_line,
            start_col: check.start_col,
            end_line: check.end_line,
            end_col: check.end_col,
            name: Some(check.name.to_string()),
        },
        message: format!(
            "{} `{}` name is too {} ({} chars, {}: {})",
            check.kind,
            check.name,
            qualifier,
            check.name.len(),
            bound_label,
            limit
        ),
        suggested_refactorings: vec!["Rename Method".into()],
        ..Default::default()
    })
}
