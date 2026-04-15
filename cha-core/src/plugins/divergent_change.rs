use std::collections::HashMap;
use std::process::Command;
use std::sync::OnceLock;

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

/// Cached per-file distinct-reason counts: built once from a single `git log` call.
static REASON_CACHE: OnceLock<HashMap<String, usize>> = OnceLock::new();

fn build_reason_cache(max_commits: usize) -> HashMap<String, usize> {
    let output = Command::new("git")
        .args([
            "log",
            "--format=%s",
            "--name-only",
            &format!("-{max_commits}"),
        ])
        .output();
    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return HashMap::new(),
    };

    // Parse: alternating subject line + file list, separated by blank lines
    let mut file_scopes: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut current_scope = String::new();
    let mut in_files = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            in_files = false;
            continue;
        }
        if !in_files {
            current_scope = extract_scope(line);
            in_files = true;
        } else {
            *file_scopes
                .entry(line.to_string())
                .or_default()
                .entry(current_scope.clone())
                .or_default() += 1;
        }
    }

    file_scopes
        .into_iter()
        .map(|(file, scopes)| (file, scopes.len()))
        .collect()
}

impl Plugin for DivergentChangeAnalyzer {
    fn name(&self) -> &str {
        "divergent_change"
    }

    fn description(&self) -> &str {
        "File changed for many different reasons"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let cache = REASON_CACHE.get_or_init(|| build_reason_cache(self.max_commits));
        let path_str = ctx.file.path.to_string_lossy();
        let reasons = cache.get(path_str.as_ref()).copied().unwrap_or(0);
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
