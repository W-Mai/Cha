use crate::Finding;

/// Filter out findings suppressed by `cha:ignore` comments in source code.
///
/// Supported formats:
/// - `// cha:ignore` — suppress all rules for the next function/class
/// - `// cha:ignore rule_name` — suppress specific rule
/// - `// cha:ignore rule1,rule2` — suppress multiple rules
/// - `# cha:ignore ...` — Python style
/// - `/* cha:ignore ... */` — block comment style
pub fn filter_ignored(findings: Vec<Finding>, source: &str) -> Vec<Finding> {
    let ignores = parse_ignore_comments(source);
    if ignores.is_empty() {
        return findings;
    }
    findings
        .into_iter()
        .filter(|f| !is_suppressed(f, &ignores))
        .collect()
}

struct IgnoreDirective {
    /// Line number where the comment appears (1-based).
    line: usize,
    /// Rule names to ignore. Empty = ignore all.
    rules: Vec<String>,
}

fn parse_ignore_comments(source: &str) -> Vec<IgnoreDirective> {
    source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let trimmed = line.trim();
            let after = extract_ignore_payload(trimmed)?;
            let rules = if after.is_empty() {
                vec![]
            } else {
                after.split(',').map(|s| s.trim().to_string()).collect()
            };
            Some(IgnoreDirective { line: i + 1, rules })
        })
        .collect()
}

fn extract_ignore_payload(line: &str) -> Option<&str> {
    // // cha:ignore ... or # cha:ignore ... or /* cha:ignore ... */
    for prefix in ["//", "#", "--"] {
        if let Some(rest) = line.strip_prefix(prefix)
            && let Some(payload) = rest.trim().strip_prefix("cha:ignore")
        {
            return Some(payload.trim());
        }
    }
    if let Some(rest) = line.strip_prefix("/*") {
        let rest = rest.strip_suffix("*/").unwrap_or(rest);
        if let Some(payload) = rest.trim().strip_prefix("cha:ignore") {
            return Some(payload.trim());
        }
    }
    None
}

fn is_suppressed(finding: &Finding, ignores: &[IgnoreDirective]) -> bool {
    ignores.iter().any(|ig| {
        // Comment on the line before the finding's start, or on the same line
        let covers = ig.line == finding.location.start_line
            || ig.line + 1 == finding.location.start_line
            // Comment anywhere before the finding within 1 line gap
            || (ig.line < finding.location.start_line
                && ig.line + 1 >= finding.location.start_line);
        if !covers {
            return false;
        }
        ig.rules.is_empty() || ig.rules.iter().any(|r| r == &finding.smell_name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Location, Severity, SmellCategory};
    use std::path::PathBuf;

    fn make_finding(name: &str, start: usize) -> Finding {
        Finding {
            smell_name: name.to_string(),
            category: SmellCategory::Bloaters,
            severity: Severity::Warning,
            location: Location {
                path: PathBuf::from("test.rs"),
                start_line: start,
                end_line: start + 5,
                name: None,
            },
            message: "test".into(),
            suggested_refactorings: vec![],
        }
    }

    #[test]
    fn ignore_all_rules() {
        let src = "// cha:ignore\nfn foo() {}";
        let findings = vec![make_finding("switch_statement", 2)];
        assert!(filter_ignored(findings, src).is_empty());
    }

    #[test]
    fn ignore_specific_rule() {
        let src = "// cha:ignore switch_statement\nfn foo() {}";
        let findings = vec![
            make_finding("switch_statement", 2),
            make_finding("long_method", 2),
        ];
        let result = filter_ignored(findings, src);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].smell_name, "long_method");
    }

    #[test]
    fn ignore_multiple_rules() {
        let src = "// cha:ignore switch_statement,long_method\nfn foo() {}";
        let findings = vec![
            make_finding("switch_statement", 2),
            make_finding("long_method", 2),
        ];
        assert!(filter_ignored(findings, src).is_empty());
    }

    #[test]
    fn no_ignore_comment() {
        let src = "fn foo() {}";
        let findings = vec![make_finding("switch_statement", 1)];
        assert_eq!(filter_ignored(findings, src).len(), 1);
    }

    #[test]
    fn python_style() {
        let src = "# cha:ignore\ndef foo(): pass";
        let findings = vec![make_finding("long_method", 2)];
        assert!(filter_ignored(findings, src).is_empty());
    }
}
