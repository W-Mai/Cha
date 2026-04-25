//! Typed intimacy: two files whose function signatures exchange each other's
//! types, in both directions. A stronger signal than bidirectional imports
//! alone — those can happen for utility reasons, whereas each file literally
//! accepting/returning a type *defined* in the other indicates that the pair
//! is functionally fused at the type level.
//!
//! Distinct from `inappropriate_intimacy` (which looks only at import graph
//! cycles at file granularity) — this looks at declared type flow and needs
//! parsed signatures, so it runs as a post-analysis pass.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory};

use crate::project_index::ProjectIndex;

const SMELL: &str = "typed_intimacy";
const MIN_TYPES_SHARED: usize = 1;

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let class_home = index.class_home();
    let models = index.models();
    // For each file: set of other files whose classes it references in its
    // function signatures.
    let uses_classes_of = build_usage_graph(models, class_home);

    let mut pairs_reported: HashSet<(PathBuf, PathBuf)> = HashSet::new();
    let mut findings = Vec::new();
    for (path_a, targets) in &uses_classes_of {
        for path_b in targets {
            let Some(reverse) = uses_classes_of.get(path_b) else {
                continue;
            };
            if !reverse.contains(path_a) {
                continue;
            }
            let key = pair_key(path_a, path_b);
            if !pairs_reported.insert(key) {
                continue;
            }
            let (shared_ab, shared_ba) = (
                shared_type_names(models, path_a, path_b, class_home),
                shared_type_names(models, path_b, path_a, class_home),
            );
            if shared_ab.len() < MIN_TYPES_SHARED || shared_ba.len() < MIN_TYPES_SHARED {
                continue;
            }
            findings.push(build_finding(path_a, path_b, &shared_ab, &shared_ba));
            findings.push(build_finding(path_b, path_a, &shared_ba, &shared_ab));
        }
    }
    findings
}

fn pair_key(a: &Path, b: &Path) -> (PathBuf, PathBuf) {
    if a <= b {
        (a.to_path_buf(), b.to_path_buf())
    } else {
        (b.to_path_buf(), a.to_path_buf())
    }
}

/// For each file, compute the set of *other* files whose declared classes it
/// references in any function signature (parameter or return type).
fn build_usage_graph(
    models: &[(PathBuf, cha_core::SourceModel)],
    class_home: &HashMap<String, PathBuf>,
) -> HashMap<PathBuf, HashSet<PathBuf>> {
    let mut graph: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();
    for (path, model) in models {
        let entry = graph.entry(path.clone()).or_default();
        for f in &model.functions {
            collect_refs(f, class_home, path, entry);
        }
    }
    graph
}

fn collect_refs(
    f: &FunctionInfo,
    class_home: &HashMap<String, PathBuf>,
    self_path: &Path,
    entry: &mut HashSet<PathBuf>,
) {
    for t in &f.parameter_types {
        if let Some(home) = class_home.get(&t.name)
            && home.as_path() != self_path
        {
            entry.insert(home.clone());
        }
    }
    if let Some(ret) = &f.return_type
        && let Some(home) = class_home.get(&ret.name)
        && home.as_path() != self_path
    {
        entry.insert(home.clone());
    }
}

/// Type names declared in `owner` that appear in `user`'s function signatures.
fn shared_type_names(
    models: &[(PathBuf, cha_core::SourceModel)],
    user: &Path,
    owner: &Path,
    class_home: &HashMap<String, PathBuf>,
) -> Vec<String> {
    let Some(user_model) = models.iter().find(|(p, _)| p == user).map(|(_, m)| m) else {
        return Vec::new();
    };
    let mut names: HashSet<String> = HashSet::new();
    for f in &user_model.functions {
        for t in &f.parameter_types {
            if class_home.get(&t.name).map(PathBuf::as_path) == Some(owner) {
                names.insert(t.name.clone());
            }
        }
        if let Some(ret) = &f.return_type
            && class_home.get(&ret.name).map(PathBuf::as_path) == Some(owner)
        {
            names.insert(ret.name.clone());
        }
    }
    let mut sorted: Vec<String> = names.into_iter().collect();
    sorted.sort();
    sorted
}

fn build_finding(path: &Path, other: &Path, from_other: &[String], to_other: &[String]) -> Finding {
    let rel_other = other.display();
    let message = format!(
        "File `{}` exchanges types with `{}` in both directions (imports `{}`, exports `{}`) — typed intimacy",
        path.display(),
        rel_other,
        from_other.join(", "),
        to_other.join(", "),
    );
    Finding {
        smell_name: SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: Location {
            path: path.to_path_buf(),
            start_line: 1,
            end_line: 1,
            ..Default::default()
        },
        message,
        suggested_refactorings: vec![
            "Extract a shared domain module that both files depend on".into(),
            "Invert one direction via a trait/interface so only one file knows the other".into(),
        ],
        actual_value: Some(from_other.len() as f64),
        threshold: Some(MIN_TYPES_SHARED as f64),
        risk_score: None,
    }
}

#[cfg(test)]
mod tests;
