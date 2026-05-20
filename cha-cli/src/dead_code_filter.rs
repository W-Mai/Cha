//! Cross-file dead_code post-filter.
//!
//! The per-file `dead_code` plugin uses single-file text search to decide if a
//! non-exported function/class is referenced. This misses two common patterns:
//!
//! 1. **Cross-file references** — a `static` helper in `foo.c` called only from
//!    `foo.c` is fine, but a `static` callback registered into another file's
//!    dispatch table will text-match negative.
//! 2. **Project-wide call graph** — every parser already extracts
//!    `FunctionInfo.called_functions`. Aggregating across all files gives a
//!    global "is anything calling this name" signal at near-zero extra cost.
//!
//! This pass runs *after* the per-file plugin emits findings, before
//! aggregation. It drops `dead_code` findings whose target name is referenced
//! from any other file in the project. The plugin's own single-file decision
//! is preserved for cases where there's no cross-file evidence either way.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use cha_core::Finding;
#[cfg(test)]
use cha_core::SourceModel;

/// Build a set of names referenced anywhere in the project.
/// Used to filter out per-file `dead_code` findings whose target is actually
/// called from another file.
#[cfg(test)]
pub fn build_cross_file_call_set(models: &[(PathBuf, SourceModel)]) -> HashSet<String> {
    let mut names = HashSet::new();
    for (_, model) in models {
        for f in &model.functions {
            for callee in &f.called_functions {
                names.insert(callee.clone());
            }
        }
    }
    names
}

/// Build the call-set by parsing all files via the shared cache. Use this
/// from `cmd_analyze` where the cache and file list are available.
pub fn build_call_set_from_files(
    files: &[PathBuf],
    cwd: &Path,
    cache: &std::sync::Mutex<cha_core::ProjectCache>,
) -> HashSet<String> {
    let mut names = HashSet::new();
    for path in files {
        if let Ok(mut c) = cache.lock()
            && let Some((_, model)) = crate::cached_parse(path, &mut c, cwd)
        {
            for f in &model.functions {
                for callee in &f.called_functions {
                    names.insert(callee.clone());
                }
            }
        }
    }
    names
}

/// Drop `dead_code` findings whose target name appears in the cross-file
/// call set. Other findings pass through untouched.
pub fn filter_dead_code(
    findings: Vec<Finding>,
    cross_file_calls: &HashSet<String>,
) -> Vec<Finding> {
    findings
        .into_iter()
        .filter(|f| {
            if f.smell_name != "dead_code" {
                return true;
            }
            match f.location.name.as_deref() {
                Some(name) => !cross_file_calls.contains(name),
                None => true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cha_core::{FunctionInfo, Location, Severity, SmellCategory};

    fn dead_code_finding(name: &str) -> Finding {
        Finding {
            smell_name: "dead_code".into(),
            category: SmellCategory::Dispensables,
            severity: Severity::Hint,
            location: Location {
                path: PathBuf::from("foo.rs"),
                start_line: 1,
                end_line: 1,
                start_col: 0,
                end_col: 0,
                name: Some(name.to_string()),
            },
            message: "dead".into(),
            ..Default::default()
        }
    }

    fn model_calling(callees: &[&str]) -> SourceModel {
        SourceModel {
            language: "rust".into(),
            functions: vec![FunctionInfo {
                name: "caller".into(),
                start_line: 1,
                end_line: 5,
                line_count: 5,
                called_functions: callees.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn filters_finding_when_called_elsewhere() {
        let models = vec![(PathBuf::from("a.rs"), model_calling(&["helper"]))];
        let calls = build_cross_file_call_set(&models);
        let findings = vec![dead_code_finding("helper")];
        let filtered = filter_dead_code(findings, &calls);
        assert!(filtered.is_empty(), "helper is called → should be filtered");
    }

    #[test]
    fn keeps_finding_when_no_cross_file_caller() {
        let models = vec![(PathBuf::from("a.rs"), model_calling(&["something_else"]))];
        let calls = build_cross_file_call_set(&models);
        let findings = vec![dead_code_finding("helper")];
        let filtered = filter_dead_code(findings, &calls);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn keeps_non_dead_code_findings() {
        let models = vec![(PathBuf::from("a.rs"), model_calling(&[]))];
        let calls = build_cross_file_call_set(&models);
        let mut other = dead_code_finding("x");
        other.smell_name = "long_method".into();
        let filtered = filter_dead_code(vec![other], &calls);
        assert_eq!(filtered.len(), 1, "non-dead_code findings unaffected");
    }

    #[test]
    fn missing_name_keeps_finding() {
        let models = vec![(PathBuf::from("a.rs"), model_calling(&[]))];
        let calls = build_cross_file_call_set(&models);
        let mut nameless = dead_code_finding("x");
        nameless.location.name = None;
        let filtered = filter_dead_code(vec![nameless], &calls);
        assert_eq!(filtered.len(), 1);
    }
}
