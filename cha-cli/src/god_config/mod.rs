//! God Config: a single "ambient" configuration type gets passed as a
//! parameter to many functions across many files. Each caller ends up
//! knowing about the whole config object (even to use one field) and
//! threading it through its own signature for the next layer — the
//! classic "Context" / "AppState" / "Settings" god-parameter pattern.
//!
//! Detection is signature-only: count how many distinct functions take
//! the type and across how many files. Above a threshold on both, the
//! type is a god config.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::{Finding, Location, Severity, SmellCategory};

use crate::project_index::ProjectIndex;

const SMELL: &str = "god_config";
const MIN_DISTINCT_CALLERS: usize = 10;
const MIN_FILES_SPANNED: usize = 3;

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let usages = collect_config_usages(index);
    usages
        .into_iter()
        .filter_map(|(type_name, sites)| build_finding_if_god(&type_name, &sites, index))
        .collect()
}

struct UsageSite {
    path: PathBuf,
    function_name: String,
}

fn collect_config_usages(index: &ProjectIndex) -> HashMap<String, Vec<UsageSite>> {
    let mut usages: HashMap<String, Vec<UsageSite>> = HashMap::new();
    for (path, model) in index.models() {
        for f in &model.functions {
            for t in &f.parameter_types {
                if !is_config_shaped(&t.name) {
                    continue;
                }
                usages.entry(t.name.clone()).or_default().push(UsageSite {
                    path: path.clone(),
                    function_name: f.name.clone(),
                });
            }
        }
    }
    usages
}

/// A type is "config-shaped" if its name matches a naming convention known
/// to carry ambient configuration / dependencies. Kept intentionally narrow
/// — `Config`/`Settings`/`Options` are the strong signals; `Context`/`Env`/
/// `AppState`/`Store` are the weaker ones that still indicate ambient intent.
fn is_config_shaped(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "Config", "Settings", "Options", "Context", "Env", "AppState", "Store",
    ];
    const SUFFIXES: &[&str] = &["Config", "Settings", "Options"];
    EXACT.contains(&name) || SUFFIXES.iter().any(|s| name.ends_with(s) && name != *s)
}

fn build_finding_if_god(
    type_name: &str,
    sites: &[UsageSite],
    index: &ProjectIndex,
) -> Option<Finding> {
    if sites.len() < MIN_DISTINCT_CALLERS {
        return None;
    }
    let files: HashSet<&Path> = sites.iter().map(|s| s.path.as_path()).collect();
    if files.len() < MIN_FILES_SPANNED {
        return None;
    }
    // Locate the type's declaring file so the finding anchors somewhere
    // meaningful — the god config itself, not one random caller.
    let declared_in = index.class_home().get(type_name).cloned();
    let sample: Vec<&str> = sites
        .iter()
        .take(3)
        .map(|s| s.function_name.as_str())
        .collect();
    Some(build_finding(
        type_name,
        sites.len(),
        files.len(),
        declared_in,
        &sample,
    ))
}

fn build_finding(
    type_name: &str,
    caller_count: usize,
    file_count: usize,
    declared_in: Option<PathBuf>,
    sample_callers: &[&str],
) -> Finding {
    let anchor = declared_in.clone().unwrap_or_else(|| PathBuf::from("."));
    let more = if caller_count > sample_callers.len() {
        format!(" (+{} more)", caller_count - sample_callers.len())
    } else {
        String::new()
    };
    let message = format!(
        "`{}` is threaded through {} functions across {} files — e.g. `{}`{}; ambient configuration is leaking everywhere, pass only the specific values each function needs",
        type_name,
        caller_count,
        file_count,
        sample_callers.join("`, `"),
        more,
    );
    Finding {
        smell_name: SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: Location {
            path: anchor,
            start_line: 1,
            end_line: 1,
            name: Some(type_name.to_string()),
            ..Default::default()
        },
        message,
        suggested_refactorings: vec![
            format!(
                "Break `{}` into smaller focused types; each caller takes only the fields it actually uses",
                type_name
            ),
            "Introduce a Parameter Object only where multiple related fields cluster together".into(),
            "For long-lived shared state, consider a dependency-injection seam instead of threading a bag through every signature".into(),
        ],
        actual_value: Some(caller_count as f64),
        threshold: Some(MIN_DISTINCT_CALLERS as f64),
        risk_score: None,
    }
}

#[cfg(test)]
mod tests;
