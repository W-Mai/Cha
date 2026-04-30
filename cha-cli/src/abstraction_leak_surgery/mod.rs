//! abstraction_leak_surgery — files that co-change in git history **and**
//! share external type references. Upgrade of the classic `shotgun_surgery`:
//! instead of just "these files change together", this asks "these files
//! change together *because* they all touch the same third-party type".
//! The shared external type is the concrete abstraction leak driving the
//! co-change — upgrading `serde_json` should not ripple across 8 files.
//!
//! Inputs:
//! - ProjectIndex (for each file's external type set, via SourceModel)
//! - `git log --name-only` (for co-change counts)
//!
//! Emits one finding per co-change pair that shares ≥ 1 external type.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use cha_core::{Finding, Location, Severity, SmellCategory, TypeOrigin};

use crate::project_index::ProjectIndex;

const SMELL: &str = "abstraction_leak_surgery";
const MIN_CO_CHANGES: usize = 5;
const MAX_COMMITS: usize = 100;

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let workspace_crates = workspace_crate_names(index);
    let external_types = build_external_type_index(index, &workspace_crates);
    if external_types.is_empty() {
        return Vec::new();
    }
    let co_changes = co_change_counts(MAX_COMMITS);
    let mut findings = Vec::new();
    let mut seen: HashSet<(PathBuf, PathBuf)> = HashSet::new();
    for ((a, b), count) in &co_changes {
        if *count < MIN_CO_CHANGES {
            continue;
        }
        let key = pair_key(a, b);
        if !seen.insert(key) {
            continue;
        }
        let Some(types_a) = external_types.get(a.as_path()) else {
            continue;
        };
        let Some(types_b) = external_types.get(b.as_path()) else {
            continue;
        };
        let shared: Vec<String> = types_a
            .intersection(types_b)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .take(5)
            .collect();
        if shared.is_empty() {
            continue;
        }
        findings.push(build_finding(a, b, &shared, *count));
    }
    findings
}

/// Set of `"module::TypeName"` strings that each file references through
/// its function signatures (parameter or return types). Only
/// `TypeOrigin::External` counts — and only modules outside this
/// workspace. Sibling project crates (e.g. `cha_core` inside Cha itself)
/// aren't the "third-party boundary" we want to flag; they're internal
/// dependencies.
fn build_external_type_index(
    index: &ProjectIndex,
    workspace_crates: &HashSet<String>,
) -> HashMap<PathBuf, HashSet<String>> {
    let mut out: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    for (path, model) in index.models() {
        let mut set = HashSet::new();
        for f in &model.functions {
            for t in &f.parameter_types {
                if let Some(entry) = external_entry(t, workspace_crates) {
                    set.insert(entry);
                }
            }
            if let Some(rt) = &f.return_type
                && let Some(entry) = external_entry(rt, workspace_crates)
            {
                set.insert(entry);
            }
        }
        if !set.is_empty() {
            out.insert(path.clone(), set);
        }
    }
    out
}

fn external_entry(t: &cha_core::TypeRef, workspace_crates: &HashSet<String>) -> Option<String> {
    let TypeOrigin::External(module) = &t.origin else {
        return None;
    };
    let root = module.split("::").next().unwrap_or(module);
    if workspace_crates.contains(root) {
        return None;
    }
    Some(format!("{module}::{}", t.name))
}

fn workspace_crate_names(index: &ProjectIndex) -> HashSet<String> {
    let mut names = HashSet::new();
    for (path, _) in index.models() {
        let Some(first) = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .find(|s| *s != "." && *s != "..")
        else {
            continue;
        };
        names.insert(first.replace('-', "_"));
    }
    names
}

/// Parse `git log --name-only -N` into per-file-pair co-change counts.
fn co_change_counts(max_commits: usize) -> HashMap<(PathBuf, PathBuf), usize> {
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
        _ => return HashMap::new(),
    };
    let commits = split_commit_groups(&text);
    let mut counts: HashMap<(PathBuf, PathBuf), usize> = HashMap::new();
    for files in &commits {
        for (i, a) in files.iter().enumerate() {
            for b in &files[i + 1..] {
                *counts.entry(pair_key(a, b)).or_default() += 1;
            }
        }
    }
    counts
}

fn split_commit_groups(text: &str) -> Vec<Vec<PathBuf>> {
    let mut commits: Vec<Vec<PathBuf>> = Vec::new();
    let mut current: Vec<PathBuf> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !current.is_empty() {
                commits.push(std::mem::take(&mut current));
            }
        } else {
            current.push(PathBuf::from(line));
        }
    }
    if !current.is_empty() {
        commits.push(current);
    }
    commits
}

fn pair_key(a: &Path, b: &Path) -> (PathBuf, PathBuf) {
    if a <= b {
        (a.to_path_buf(), b.to_path_buf())
    } else {
        (b.to_path_buf(), a.to_path_buf())
    }
}

fn build_finding(a: &Path, b: &Path, shared_types: &[String], count: usize) -> Finding {
    let types_list = shared_types
        .iter()
        .map(|s| format!("`{s}`"))
        .collect::<Vec<_>>()
        .join(", ");
    Finding {
        smell_name: SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: Location {
            path: a.to_path_buf(),
            start_line: 1,
            end_line: 1,
            ..Default::default()
        },
        message: format!(
            "`{}` and `{}` co-changed in {count} commits and share external type(s) {types_list} — upgrading the external module ripples through both files; consider an Adapter that encapsulates the external surface in one place",
            a.display(),
            b.display(),
        ),
        suggested_refactorings: vec![
            "Extract an Adapter module that wraps the shared external types and expose a project-local API to both sites".into(),
            "If only one file truly needs the external type, move the references there and call it from the other".into(),
        ],
        actual_value: Some(count as f64),
        threshold: Some(MIN_CO_CHANGES as f64),
        risk_score: None,
    }
}

#[cfg(test)]
mod tests;
