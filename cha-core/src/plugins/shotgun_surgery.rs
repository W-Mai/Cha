use std::collections::HashMap;
use std::process::Command;

use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect files that always change together (shotgun surgery).
/// Uses git log to find co-change patterns.
pub struct ShotgunSurgeryAnalyzer {
    pub min_co_changes: usize,
    pub max_commits: usize,
}

impl Default for ShotgunSurgeryAnalyzer {
    fn default() -> Self {
        Self {
            min_co_changes: 5,
            max_commits: 100,
        }
    }
}

impl Plugin for ShotgunSurgeryAnalyzer {
    fn name(&self) -> &str {
        "shotgun_surgery"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let path_str = ctx.file.path.to_string_lossy();
        let co_changes = git_co_changes(&path_str, self.max_commits);
        co_changes
            .into_iter()
            .filter(|(_, count)| *count >= self.min_co_changes)
            .map(|(other, count)| Finding {
                smell_name: "shotgun_surgery".into(),
                category: SmellCategory::ChangePreventers,
                severity: Severity::Hint,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: 1,
                    end_line: ctx.model.total_lines,
                    name: None,
                },
                message: format!(
                    "`{}` changed together with `{}` in {} commits, consider Move Method/Field",
                    path_str, other, count
                ),
                suggested_refactorings: vec!["Move Method".into(), "Move Field".into()],
            })
            .collect()
    }
}

/// Get files that frequently change together with the given file.
fn git_co_changes(file: &str, max_commits: usize) -> Vec<(String, usize)> {
    let output = Command::new("git")
        .args([
            "log",
            "--pretty=format:%H",
            &format!("-{max_commits}"),
            "--",
            file,
        ])
        .output();
    let commits: Vec<String> = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(String::from)
            .collect(),
        _ => return vec![],
    };
    let mut co_change_count: HashMap<String, usize> = HashMap::new();
    for commit in &commits {
        let files_output = Command::new("git")
            .args(["diff-tree", "--no-commit-id", "--name-only", "-r", commit])
            .output();
        if let Ok(o) = files_output {
            for f in String::from_utf8_lossy(&o.stdout).lines() {
                if f != file {
                    *co_change_count.entry(f.to_string()).or_default() += 1;
                }
            }
        }
    }
    let mut result: Vec<_> = co_change_count.into_iter().collect();
    result.sort_by_key(|a| std::cmp::Reverse(a.1));
    result.truncate(3); // Top 3 co-changed files
    result
}
