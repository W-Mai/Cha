use std::collections::HashSet;
use std::process::Command;

use cha_core::{Finding, Location, Severity, SmellCategory};
use std::path::PathBuf;

/// Detect tangled changes: commits that touch unrelated top-level directories.
///
/// ## References
///
/// [1] A. Tornhill, "Your Code as a Crime Scene," Pragmatic Bookshelf, 2015.
pub fn detect_tangled(count: usize, threshold: usize) -> Vec<Finding> {
    let commits = recent_commits_with_files(count);
    commits
        .into_iter()
        .filter_map(|(hash, msg, dirs)| {
            (dirs.len() >= threshold).then(|| Finding {
                smell_name: "tangled_change".into(),
                category: SmellCategory::ChangePreventers,
                severity: Severity::Hint,
                location: Location {
                    path: PathBuf::from(&hash[..7]),
                    start_line: 1,
                    end_line: 1,
                    name: Some(hash[..7].to_string()),
                    ..Default::default()
                },
                message: format!(
                    "Commit {} touches {} directories ({}) — {}",
                    &hash[..7],
                    dirs.len(),
                    dirs.into_iter().collect::<Vec<_>>().join(", "),
                    msg
                ),
                suggested_refactorings: vec!["Split into focused commits".into()],
                ..Default::default()
            })
        })
        .collect()
}

// cha:ignore high_complexity,cognitive_complexity
fn recent_commits_with_files(n: usize) -> Vec<(String, String, HashSet<String>)> {
    let output = Command::new("git")
        .args(["log", "--format=%H %s", "--name-only", "-n", &n.to_string()])
        .output()
        .ok();
    let Some(output) = output else { return vec![] };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut result = Vec::new();
    let mut current: Option<(String, String, HashSet<String>)> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() >= 40
            && line
                .chars()
                .take(40)
                .all(|c| c.is_ascii_hexdigit() || c == ' ')
        {
            if let Some(prev) = current.take() {
                result.push(prev);
            }
            let (hash, msg) = line.split_once(' ').unwrap_or((line, ""));
            current = Some((hash.to_string(), msg.to_string(), HashSet::new()));
        } else if let Some((_, _, ref mut dirs)) = current
            && let Some(top) = line.split('/').next()
            && !top.contains('.')
        {
            dirs.insert(top.to_string());
        }
    }
    if let Some(last) = current {
        result.push(last);
    }
    result
}
