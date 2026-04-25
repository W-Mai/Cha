//! Circular abstraction: file A's functions call functions defined in file B,
//! and B's functions call back into A. The two files form a mutual
//! dependency at the behaviour level, not just the import level — a sign
//! that the abstraction boundary between them isn't doing its job.
//!
//! Differs from `typed_intimacy` (which looks at type flow across
//! signatures) — this looks at the call graph itself. Catches pairs that
//! exchange behaviour without necessarily exchanging domain types.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::{Finding, Location, Severity, SmellCategory};

use crate::project_index::ProjectIndex;

const SMELL: &str = "circular_abstraction";
const MIN_CALLS_EACH_SIDE: usize = 2;

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let call_counts = build_cross_file_call_counts(index);
    let pairs = find_cycles(&call_counts);
    let mut findings = Vec::new();
    let mut reported: HashSet<(PathBuf, PathBuf)> = HashSet::new();
    for (a, b, a_to_b, b_to_a) in pairs {
        let key = pair_key(&a, &b);
        if !reported.insert(key) {
            continue;
        }
        findings.push(build_finding(&a, &b, a_to_b, b_to_a));
        findings.push(build_finding(&b, &a, b_to_a, a_to_b));
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

/// For each (caller_file, callee_file) pair, count how many calls flow
/// caller → callee across all functions in caller. Intra-file calls are
/// excluded; unresolved callees (stdlib, external) are ignored.
fn build_cross_file_call_counts(index: &ProjectIndex) -> HashMap<(PathBuf, PathBuf), usize> {
    let fn_home = index.function_home();
    let mut counts: HashMap<(PathBuf, PathBuf), usize> = HashMap::new();
    for (caller_path, model) in index.models() {
        for f in &model.functions {
            for callee in &f.called_functions {
                let Some(callee_home) = fn_home.get(callee) else {
                    continue;
                };
                if callee_home.as_path() == caller_path.as_path() {
                    continue;
                }
                *counts
                    .entry((caller_path.clone(), callee_home.clone()))
                    .or_default() += 1;
            }
        }
    }
    counts
}

fn find_cycles(
    counts: &HashMap<(PathBuf, PathBuf), usize>,
) -> Vec<(PathBuf, PathBuf, usize, usize)> {
    let mut cycles = Vec::new();
    for ((a, b), a_to_b) in counts {
        if *a_to_b < MIN_CALLS_EACH_SIDE {
            continue;
        }
        let Some(b_to_a) = counts.get(&(b.clone(), a.clone())) else {
            continue;
        };
        if *b_to_a < MIN_CALLS_EACH_SIDE {
            continue;
        }
        cycles.push((a.clone(), b.clone(), *a_to_b, *b_to_a));
    }
    cycles
}

fn build_finding(side: &Path, other: &Path, out_calls: usize, in_calls: usize) -> Finding {
    let message = format!(
        "File `{}` has {} calls into `{}` which in turn makes {} calls back — circular abstraction; the pair shares behaviour in both directions",
        side.display(),
        out_calls,
        other.display(),
        in_calls,
    );
    Finding {
        smell_name: SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: Location {
            path: side.to_path_buf(),
            start_line: 1,
            end_line: 1,
            ..Default::default()
        },
        message,
        suggested_refactorings: vec![
            "Extract a shared layer both files depend on".into(),
            "Invert one direction via a trait/interface so only one side holds the dependency"
                .into(),
        ],
        actual_value: Some(out_calls as f64),
        threshold: Some(MIN_CALLS_EACH_SIDE as f64),
        risk_score: None,
    }
}

#[cfg(test)]
mod tests;
