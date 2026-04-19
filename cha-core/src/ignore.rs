use crate::Finding;

/// Filter out findings suppressed by `cha:ignore` comments in source code,
/// and re-evaluate findings against `cha:set` threshold overrides.
///
/// Supported formats:
/// - `// cha:ignore` — suppress all rules for the next function/class
/// - `// cha:ignore rule_name` — suppress specific rule
/// - `// cha:ignore rule1,rule2` — suppress multiple rules
/// - `// cha:set threshold=100` — override threshold for all rules on next item
/// - `// cha:set rule_name=100` — override threshold for specific rule
/// - `# cha:ignore ...` / `# cha:set ...` — Python style
/// - `/* cha:ignore ... */` / `/* cha:set ... */` — block comment style
pub fn filter_ignored(findings: Vec<Finding>, source: &str) -> Vec<Finding> {
    let ignores = parse_ignore_comments(source);
    let overrides = parse_set_comments(source);
    if ignores.is_empty() && overrides.is_empty() {
        return findings;
    }
    findings
        .into_iter()
        .filter(|f| !is_suppressed(f, &ignores))
        .filter(|f| !is_overridden_away(f, &overrides))
        .collect()
}

struct IgnoreDirective {
    line: usize,
    rules: Vec<String>,
}

struct SetDirective {
    line: usize,
    /// Optional rule name filter. None = applies to all rules.
    rule: Option<String>,
    /// New threshold value.
    value: f64,
}

fn parse_ignore_comments(source: &str) -> Vec<IgnoreDirective> {
    source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let after = extract_directive_payload(line.trim(), "cha:ignore")?;
            let rules = if after.is_empty() {
                vec![]
            } else {
                after.split(',').map(|s| s.trim().to_string()).collect()
            };
            Some(IgnoreDirective { line: i + 1, rules })
        })
        .collect()
}

fn parse_set_comments(source: &str) -> Vec<SetDirective> {
    source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let after = extract_directive_payload(line.trim(), "cha:set")?;
            // Format: "rule_name=value" or "threshold=value"
            let (key, val_str) = after.split_once('=')?;
            let value: f64 = val_str.trim().parse().ok()?;
            let key = key.trim();
            let rule = if key == "threshold" {
                None
            } else {
                Some(key.to_string())
            };
            Some(SetDirective {
                line: i + 1,
                rule,
                value,
            })
        })
        .collect()
}

fn extract_directive_payload<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    for prefix in ["//", "#", "--"] {
        if let Some(rest) = line.strip_prefix(prefix)
            && let Some(payload) = rest.trim().strip_prefix(directive)
        {
            return Some(payload.trim());
        }
    }
    if let Some(rest) = line.strip_prefix("/*") {
        let rest = rest.strip_suffix("*/").unwrap_or(rest);
        if let Some(payload) = rest.trim().strip_prefix(directive) {
            return Some(payload.trim());
        }
    }
    None
}

fn covers(directive_line: usize, finding: &Finding) -> bool {
    // Allow directive on same line, or within a consecutive block of directives
    // immediately before the finding (e.g. two cha:ignore lines before a function)
    let start = finding.location.start_line;
    directive_line == start || (directive_line < start && start - directive_line <= 2)
}

fn is_suppressed(finding: &Finding, ignores: &[IgnoreDirective]) -> bool {
    ignores.iter().any(|ig| {
        if !covers(ig.line, finding) {
            return false;
        }
        ig.rules.is_empty() || ig.rules.iter().any(|r| r == &finding.smell_name)
    })
}

/// Check if a finding should be removed because a `cha:set` override
/// raises the threshold above the actual value.
fn is_overridden_away(finding: &Finding, overrides: &[SetDirective]) -> bool {
    let (actual, _threshold) = match (finding.actual_value, finding.threshold) {
        (Some(a), Some(t)) => (a, t),
        _ => return false, // no numeric data, can't override
    };
    overrides.iter().any(|sd| {
        if !covers(sd.line, finding) {
            return false;
        }
        let rule_matches = match &sd.rule {
            None => true,
            Some(r) => r == &finding.smell_name,
        };
        // If the new threshold is higher than actual value, suppress the finding.
        // For ratio-based thresholds (actual >= threshold triggers), same logic applies.
        rule_matches && actual < sd.value
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
            ..Default::default()
        }
    }

    fn make_finding_with_value(name: &str, start: usize, actual: f64, threshold: f64) -> Finding {
        Finding {
            actual_value: Some(actual),
            threshold: Some(threshold),
            ..make_finding(name, start)
        }
    }

    // --- cha:ignore tests ---

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

    // --- cha:set tests ---

    #[test]
    fn set_raises_threshold_suppresses() {
        // Function is 80 lines, default threshold 50, but cha:set raises to 100
        let src = "// cha:set long_method=100\nfn foo() {}";
        let findings = vec![make_finding_with_value("long_method", 2, 80.0, 50.0)];
        assert!(filter_ignored(findings, src).is_empty());
    }

    #[test]
    fn set_threshold_still_exceeded() {
        // Function is 120 lines, cha:set raises to 100, still exceeded
        let src = "// cha:set long_method=100\nfn foo() {}";
        let findings = vec![make_finding_with_value("long_method", 2, 120.0, 50.0)];
        assert_eq!(filter_ignored(findings, src).len(), 1);
    }

    #[test]
    fn set_generic_threshold() {
        // threshold=100 applies to all rules
        let src = "// cha:set threshold=100\nfn foo() {}";
        let findings = vec![make_finding_with_value("long_method", 2, 80.0, 50.0)];
        assert!(filter_ignored(findings, src).is_empty());
    }

    #[test]
    fn set_wrong_rule_no_effect() {
        let src = "// cha:set high_complexity=100\nfn foo() {}";
        let findings = vec![make_finding_with_value("long_method", 2, 80.0, 50.0)];
        assert_eq!(filter_ignored(findings, src).len(), 1);
    }

    #[test]
    fn set_no_actual_value_no_effect() {
        // Finding without actual_value is not affected by cha:set
        let src = "// cha:set long_method=100\nfn foo() {}";
        let findings = vec![make_finding("long_method", 2)];
        assert_eq!(filter_ignored(findings, src).len(), 1);
    }

    #[test]
    fn set_python_style() {
        let src = "# cha:set long_method=100\ndef foo(): pass";
        let findings = vec![make_finding_with_value("long_method", 2, 80.0, 50.0)];
        assert!(filter_ignored(findings, src).is_empty());
    }

    #[test]
    fn set_block_comment_style() {
        let src = "/* cha:set long_method=100 */\nfn foo() {}";
        let findings = vec![make_finding_with_value("long_method", 2, 80.0, 50.0)];
        assert!(filter_ignored(findings, src).is_empty());
    }

    #[test]
    fn set_does_not_affect_other_lines() {
        let src = "fn bar() {}\n// cha:set long_method=100\nfn foo() {}";
        let findings = vec![
            make_finding_with_value("long_method", 1, 80.0, 50.0), // bar — not covered
            make_finding_with_value("long_method", 3, 80.0, 50.0), // foo — covered
        ];
        let result = filter_ignored(findings, src);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location.start_line, 1);
    }
}
