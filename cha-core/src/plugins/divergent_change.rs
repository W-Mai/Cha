use std::collections::HashMap;
use std::process::Command;

use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect files changed for many different reasons (divergent change).
/// Uses git log to count distinct change "clusters" by commit message keywords.
pub struct DivergentChangeAnalyzer {
    pub min_distinct_reasons: usize,
    pub max_commits: usize,
}

impl Default for DivergentChangeAnalyzer {
    fn default() -> Self {
        Self {
            min_distinct_reasons: 4,
            max_commits: 50,
        }
    }
}

impl Plugin for DivergentChangeAnalyzer {
    fn name(&self) -> &str {
        "divergent_change"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let path_str = ctx.file.path.to_string_lossy();
        let reasons = git_change_reasons(&path_str, self.max_commits);
        if reasons < self.min_distinct_reasons {
            return vec![];
        }
        vec![Finding {
            smell_name: "divergent_change".into(),
            category: SmellCategory::ChangePreventers,
            severity: Severity::Hint,
            location: Location {
                path: ctx.file.path.clone(),
                start_line: 1,
                end_line: ctx.model.total_lines,
                name: None,
            },
            message: format!(
                "`{}` was changed for ~{} distinct reasons in last {} commits, consider Extract Class",
                path_str, reasons, self.max_commits
            ),
            suggested_refactorings: vec!["Extract Class".into()],
        }]
    }
}

/// Estimate distinct change reasons by extracting scope prefixes from commit messages.
fn git_change_reasons(file: &str, max_commits: usize) -> usize {
    let output = Command::new("git")
        .args([
            "log",
            "--pretty=format:%s",
            &format!("-{max_commits}"),
            "--",
            file,
        ])
        .output();
    let messages: Vec<String> = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(String::from)
            .collect(),
        _ => return 0,
    };
    // Extract scope from conventional commit format: "type(scope): ..."
    let mut scopes: HashMap<String, usize> = HashMap::new();
    for msg in &messages {
        let scope = extract_scope(msg);
        *scopes.entry(scope).or_default() += 1;
    }
    scopes.len()
}

/// Extract scope/category from a commit message.
fn extract_scope(msg: &str) -> String {
    // Try conventional commit: "type(scope): ..."
    if let Some(start) = msg.find('(')
        && let Some(end) = msg[start..].find(')')
    {
        return msg[start + 1..start + end].to_lowercase();
    }
    // Fallback: first word
    msg.split_whitespace()
        .next()
        .unwrap_or("unknown")
        .to_lowercase()
}
