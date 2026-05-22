use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};
use regex::Regex;
use std::sync::LazyLock;

pub struct HardcodedSecretAnalyzer;

impl Default for HardcodedSecretAnalyzer {
    fn default() -> Self {
        Self
    }
}

static PATTERNS: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    // String-literal nodes are fed in pre-stripped, so patterns match raw secrets.
    [
        ("AWS Access Key", r#"(?i)AKIA[0-9A-Z]{16,}"#),
        (
            "Private Key",
            r#"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----"#,
        ),
        ("GitHub Token", r#"gh[ps]_[A-Za-z0-9_]{36,}"#),
        ("Slack Token", r#"xox[bpors]-[A-Za-z0-9-]{10,}"#),
        (
            "JWT",
            r#"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"#,
        ),
        ("Hex Secret", r#"^[0-9a-fA-F]{32,}$"#),
        ("Long Base64-ish Secret", r#"^[A-Za-z0-9+/=_-]{40,}$"#),
    ]
    .iter()
    .map(|(name, pat)| (*name, Regex::new(pat).unwrap()))
    .collect()
});

impl Plugin for HardcodedSecretAnalyzer {
    fn name(&self) -> &str {
        "hardcoded_secret"
    }

    fn smells(&self) -> Vec<String> {
        vec!["hardcoded_secret".into()]
    }

    fn description(&self) -> &str {
        "Hardcoded API keys, tokens, passwords"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let (Some(tree), Some(lang)) = (ctx.tree, ctx.ts_language) else {
            return Vec::new();
        };
        let source = ctx.file.content.as_bytes();

        // Pull every string-literal node. Each grammar names them slightly
        // differently — query for the union; misses on a grammar are silent
        // (run_query returns empty for invalid patterns).
        let queries = [
            "(string_literal) @s",
            "(raw_string_literal) @s",
            "(interpreted_string_literal) @s",
            "(string) @s",
            "(string_fragment) @s",
        ];
        let mut findings = Vec::new();
        for q in queries {
            for matches in crate::query::run_query(tree, lang, source, q) {
                for cap in matches {
                    if cap.capture_name != "s" {
                        continue;
                    }
                    if let Some((label, _)) = pick_pattern(&cap.text) {
                        findings.push(make_finding(ctx, &cap, label));
                    }
                }
            }
        }
        findings
    }
}

fn pick_pattern(text: &str) -> Option<(&'static str, &Regex)> {
    let stripped = strip_quotes(text);
    for (label, re) in PATTERNS.iter() {
        if re.is_match(stripped) {
            return Some((label, re));
        }
    }
    None
}

fn strip_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

fn make_finding(ctx: &AnalysisContext, cap: &crate::query::QueryMatch, label: &str) -> Finding {
    Finding {
        smell_name: "hardcoded_secret".into(),
        category: SmellCategory::Security,
        severity: Severity::Warning,
        location: Location {
            path: ctx.file.path.clone(),
            start_line: cap.start_line as usize,
            start_col: cap.start_col as usize,
            end_line: cap.end_line as usize,
            end_col: cap.end_col as usize,
            name: Some(label.to_string()),
        },
        message: format!("Possible hardcoded {label} detected"),
        suggested_refactorings: vec![
            "Use environment variables".into(),
            "Use a secrets manager".into(),
        ],
        ..Default::default()
    }
}
