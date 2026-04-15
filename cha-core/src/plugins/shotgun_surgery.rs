use std::collections::HashMap;
use std::process::Command;
use std::sync::OnceLock;

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

/// Cached co-change data: built once from a single `git log` call.
static CO_CHANGE_CACHE: OnceLock<HashMap<String, Vec<(String, usize)>>> = OnceLock::new();

fn build_co_change_cache(max_commits: usize) -> HashMap<String, Vec<(String, usize)>> {
    let commits = parse_commit_file_groups(max_commits);
    let mut per_file: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for files in &commits {
        for f in files {
            for other in files {
                if f != other {
                    *per_file
                        .entry(f.clone())
                        .or_default()
                        .entry(other.clone())
                        .or_default() += 1;
                }
            }
        }
    }
    per_file
        .into_iter()
        .map(|(file, counts)| {
            let mut top: Vec<_> = counts.into_iter().collect();
            top.sort_by_key(|a| std::cmp::Reverse(a.1));
            top.truncate(3);
            (file, top)
        })
        .collect()
}

fn parse_commit_file_groups(max_commits: usize) -> Vec<Vec<String>> {
    let output = Command::new("git")
        .args([
            "log",
            "--pretty=format:",
            "--name-only",
            &format!("-{max_commits}"),
        ])
        .output();
    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };
    let mut commits = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !current.is_empty() {
                commits.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line.to_string());
        }
    }
    if !current.is_empty() {
        commits.push(current);
    }
    commits
}

impl Plugin for ShotgunSurgeryAnalyzer {
    fn name(&self) -> &str {
        "shotgun_surgery"
    }

    fn description(&self) -> &str {
        "Files that always change together"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let cache = CO_CHANGE_CACHE.get_or_init(|| build_co_change_cache(self.max_commits));
        let path_str = ctx.file.path.to_string_lossy();
        let co_changes = match cache.get(path_str.as_ref()) {
            Some(v) => v,
            None => return vec![],
        };
        co_changes
            .iter()
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
