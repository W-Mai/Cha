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
    [
        ("AWS Access Key", r#"(?i)(aws_access_key_id|aws_secret_access_key)\s*[=:]\s*["']?[A-Za-z0-9/+=]{20,}"#),
        ("Generic Secret", r#"(?i)(secret|password|passwd|token|api_key|apikey|auth_token|access_token)\s*[=:]\s*["'][^"']{8,}["']"#),
        ("Private Key", r#"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----"#),
        ("GitHub Token", r#"gh[ps]_[A-Za-z0-9_]{36,}"#),
        ("Slack Token", r#"xox[bpors]-[A-Za-z0-9-]{10,}"#),
        ("JWT", r#"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"#),
        ("Hex Secret (32+)", r#"(?i)(secret|key|token|password)\s*[=:]\s*["'][0-9a-f]{32,}["']"#),
    ]
    .iter()
    .map(|(name, pat)| (*name, Regex::new(pat).unwrap()))
    .collect()
});

impl Plugin for HardcodedSecretAnalyzer {
    fn name(&self) -> &str {
        "hardcoded_secret"
    }

    fn description(&self) -> &str {
        "Hardcoded API keys, tokens, passwords"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (line_num, line) in ctx.file.content.lines().enumerate() {
            let ln = line_num + 1;
            if is_skip_line(line) {
                continue;
            }
            for (label, re) in PATTERNS.iter() {
                if re.is_match(line) {
                    findings.push(Finding {
                        smell_name: "hardcoded_secret".into(),
                        category: SmellCategory::Security,
                        severity: Severity::Warning,
                        location: Location {
                            path: ctx.file.path.clone(),
                            start_line: ln,
                            end_line: ln,
                            name: Some(label.to_string()),
                        },
                        message: format!("Possible hardcoded {label} detected"),
                        suggested_refactorings: vec![
                            "Use environment variables".into(),
                            "Use a secrets manager".into(),
                        ],
                        ..Default::default()
                    });
                    break; // one finding per line
                }
            }
        }
        findings
    }
}

fn is_skip_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.contains("example")
        || trimmed.contains("placeholder")
        || trimmed.contains("xxx")
        || trimmed.contains("TODO")
}
