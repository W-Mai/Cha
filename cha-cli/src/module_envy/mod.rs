//! Module envy: a function calls many more functions defined in some *other*
//! file than it calls in its own. The method is a "resident" of the wrong
//! module — its body does work that belongs in the envied module.
//!
//! Complements `feature_envy` (which looks at member-access within a single
//! function scope) by lifting the lens to the cross-file level: *where do my
//! called functions live?* If the answer is "mostly in another file",
//! something is misplaced.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory};

use crate::project_index::ProjectIndex;

const SMELL: &str = "module_envy";
const MIN_EXTERNAL_CALLS: usize = 3;
const MIN_RATIO_OVER_LOCAL: f64 = 2.0;

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let fn_home = index.function_home();
    let mut findings = Vec::new();
    for (path, model) in index.models() {
        for f in &model.functions {
            if let Some(finding) = check_envy(path, f, fn_home) {
                findings.push(finding);
            }
        }
    }
    findings
}

fn check_envy(
    self_path: &Path,
    f: &FunctionInfo,
    fn_home: &HashMap<String, PathBuf>,
) -> Option<Finding> {
    let mut per_file: HashMap<PathBuf, usize> = HashMap::new();
    let mut own_count: usize = 0;
    for callee in &f.called_functions {
        let Some(home) = fn_home.get(callee) else {
            continue;
        };
        if home.as_path() == self_path {
            own_count += 1;
        } else {
            *per_file.entry(home.clone()).or_default() += 1;
        }
    }
    let (top_file, top_count) = per_file.iter().max_by_key(|(_, c)| *c)?;
    if *top_count < MIN_EXTERNAL_CALLS {
        return None;
    }
    // Test files legitimately depend on shared test helpers (common.rs,
    // fixtures, builders) — that's the whole point of a test helper. Skip
    // the pair when both files smell like test code.
    if is_test_path(self_path) && is_test_path(top_file) {
        return None;
    }
    // The envied file looks like a shared-helpers module (common, util,
    // helpers, shared) — by design things depend on it without being
    // misplaced. Skip.
    if is_shared_helper_path(top_file) {
        return None;
    }
    // Ratio check: external cluster must be meaningfully larger than own-file
    // calls. A function that calls lots of everything (including locally) is
    // probably a coordinator, not envious.
    let allowed_own = (*top_count as f64 / MIN_RATIO_OVER_LOCAL).ceil() as usize;
    if own_count >= allowed_own {
        return None;
    }
    Some(build_finding(self_path, f, top_file, *top_count, own_count))
}

fn is_test_path(path: &Path) -> bool {
    const TEST_DIRS: &[&str] = &["tests", "test", "__tests__", "spec", "specs"];
    if path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .is_some_and(|s| TEST_DIRS.contains(&s))
    }) {
        return true;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    stem.starts_with("test_")
        || stem.ends_with("_test")
        || stem.ends_with(".test")
        || stem.ends_with(".spec")
        || stem.ends_with("_spec")
}

fn is_shared_helper_path(path: &Path) -> bool {
    const HELPERS: &[&str] = &[
        "common", "util", "utils", "helpers", "helper", "shared", "fixtures", "prelude",
    ];
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    HELPERS.contains(&stem.to_ascii_lowercase().as_str())
}

fn build_finding(
    self_path: &Path,
    f: &FunctionInfo,
    envied: &Path,
    external_calls: usize,
    own_calls: usize,
) -> Finding {
    let message = format!(
        "Function `{}` makes {} calls into `{}` but only {} into its own file `{}` — likely belongs in the envied module",
        f.name,
        external_calls,
        envied.display(),
        own_calls,
        self_path.display(),
    );
    Finding {
        smell_name: SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: Location {
            path: self_path.to_path_buf(),
            start_line: f.start_line,
            start_col: f.name_col,
            end_line: f.start_line,
            end_col: f.name_end_col,
            name: Some(f.name.clone()),
        },
        message,
        suggested_refactorings: vec![
            format!(
                "Move `{}` into `{}` (Move Method) — it already lives there in spirit",
                f.name,
                envied.display()
            ),
            "Or split the dependency: extract a shared abstraction both files can depend on".into(),
        ],
        actual_value: Some(external_calls as f64),
        threshold: Some(MIN_EXTERNAL_CALLS as f64),
        risk_score: None,
    }
}

#[cfg(test)]
mod tests;
