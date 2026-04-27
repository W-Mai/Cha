//! Signature-based abstraction leak detectors.
//!
//! Three smells share a detection pipeline here:
//!
//! - `abstraction_boundary_leak` — dispatcher fans out to ≥ N sibling
//!   callbacks that all take the same non-local type in corresponding
//!   parameter positions. Missing Anti-Corruption Layer on the way *in*.
//! - `return_type_leak` — dispatcher fans out to ≥ N sibling callbacks
//!   whose **return types** are all the same non-local type. Missing
//!   Anti-Corruption Layer on the way *out*.
//! - `test_only_type_in_production` — a type declared only in test files
//!   (mocks, stubs, fixtures) appears in a production signature. Signals
//!   that test scaffolding is bleeding into shipping code.
//!
//! All three share the parse-once / build-indices pipeline (`cha-parser`
//! surfaces TypeOrigin on each parameter and on the return type).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory, TypeOrigin, TypeRef};

const ABL_SMELL: &str = "abstraction_boundary_leak";
const RTL_SMELL: &str = "return_type_leak";
const TEST_ONLY_SMELL: &str = "test_only_type_in_production";
const DEFAULT_MIN_GROUP_SIZE: usize = 3;

/// Run the detector over the shared `ProjectIndex`. Returns hint/warning-
/// severity findings. All three smells emitted here (`abstraction_boundary_
/// leak`, `return_type_leak`, `test_only_type_in_production`) share the same
/// parse + index walk — the caller gates on any of them being enabled.
pub fn detect(index: &crate::project_index::ProjectIndex) -> Vec<Finding> {
    detect_from_models(index.models(), DEFAULT_MIN_GROUP_SIZE)
}

fn detect_from_models(
    models: &[(PathBuf, cha_core::SourceModel)],
    min_group_size: usize,
) -> Vec<Finding> {
    let (by_name, project_types) = build_indices(models);
    let test_only_types = build_test_only_type_set(models);
    let mut findings = Vec::new();
    for (path, model) in models {
        let in_test_file = is_test_path(path);
        for f in &model.functions {
            if !in_test_file {
                append_test_only_leaks(path, f, &test_only_types, &mut findings);
            }
            append_dispatcher_findings(
                path,
                f,
                &by_name,
                &project_types,
                min_group_size,
                &mut findings,
            );
        }
    }
    findings
}

/// Path-based heuristic for "this file is test scaffolding": common patterns
/// across Rust/TypeScript/Python/Go/C test layouts.
fn is_test_path(path: &Path) -> bool {
    if path_has_test_segment(path) {
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

/// Walk the path segment-by-segment; match any directory literally called
/// `tests`, `test`, `__tests__`, `spec`, or `specs`. Handles leading and
/// intermediate positions alike, unlike substring matching.
fn path_has_test_segment(path: &Path) -> bool {
    const TEST_DIRS: &[&str] = &["tests", "test", "__tests__", "spec", "specs"];
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .is_some_and(|s| TEST_DIRS.contains(&s))
    })
}

/// Build the set of type names declared **only in test files** — i.e. a
/// class/struct/typedef that appears in at least one test file and in no
/// production file.
fn build_test_only_type_set(models: &[(PathBuf, cha_core::SourceModel)]) -> HashSet<String> {
    let mut in_test: HashSet<String> = HashSet::new();
    let mut in_prod: HashSet<String> = HashSet::new();
    for (path, model) in models {
        let target = if is_test_path(path) {
            &mut in_test
        } else {
            &mut in_prod
        };
        for c in &model.classes {
            target.insert(c.name.clone());
        }
        for (alias, _) in &model.type_aliases {
            target.insert(alias.clone());
        }
    }
    in_test.difference(&in_prod).cloned().collect()
}

fn append_test_only_leaks(
    path: &Path,
    f: &FunctionInfo,
    test_only_types: &HashSet<String>,
    findings: &mut Vec<Finding>,
) {
    for (idx, param) in f.parameter_types.iter().enumerate() {
        if test_only_types.contains(&param.name) {
            findings.push(build_test_only_finding(path, f, &param.name, Some(idx + 1)));
        }
    }
    if let Some(ret) = &f.return_type
        && test_only_types.contains(&ret.name)
    {
        findings.push(build_test_only_finding(path, f, &ret.name, None));
    }
}

fn build_test_only_finding(
    path: &Path,
    f: &FunctionInfo,
    type_name: &str,
    param_position: Option<usize>,
) -> Finding {
    let where_it = match param_position {
        Some(i) => format!("parameter #{i}"),
        None => "return type".to_string(),
    };
    Finding {
        smell_name: TEST_ONLY_SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Warning,
        location: Location {
            path: path.to_path_buf(),
            start_line: f.start_line,
            start_col: f.name_col,
            end_line: f.start_line,
            end_col: f.name_end_col,
            name: Some(f.name.clone()),
        },
        message: format!(
            "Function `{}` uses test-only type `{}` as {}; test scaffolding is leaking into production",
            f.name, type_name, where_it
        ),
        suggested_refactorings: vec![
            "Move `{type}` out of tests into the crate it belongs to, or".into(),
            "Introduce a real production type and use the mock only in tests".into(),
        ],
        ..Default::default()
    }
}

fn append_dispatcher_findings(
    path: &Path,
    dispatcher: &FunctionInfo,
    by_name: &HashMap<&str, (&PathBuf, &FunctionInfo)>,
    project_types: &HashSet<&str>,
    min_group_size: usize,
    findings: &mut Vec<Finding>,
) {
    for group in find_sibling_groups(dispatcher, by_name, min_group_size) {
        if let Some(f) = check_param_leak(path, dispatcher, &group, min_group_size, project_types) {
            findings.push(f);
        }
        if let Some(f) = check_return_leak(path, dispatcher, &group, min_group_size, project_types)
        {
            findings.push(f);
        }
    }
}

/// Global name → function index + project-wide type registry for origin
/// fallback (types declared in sibling modules, C typedef aliases, etc.).
type IndexResult<'a> = (
    HashMap<&'a str, (&'a PathBuf, &'a FunctionInfo)>,
    HashSet<&'a str>,
);

fn build_indices(models: &[(PathBuf, cha_core::SourceModel)]) -> IndexResult<'_> {
    let mut by_name: HashMap<&str, (&PathBuf, &FunctionInfo)> = HashMap::new();
    let mut project_types: HashSet<&str> = HashSet::new();
    for (path, model) in models {
        for f in &model.functions {
            by_name.entry(f.name.as_str()).or_insert((path, f));
        }
        for c in &model.classes {
            project_types.insert(c.name.as_str());
        }
        for (alias, original) in &model.type_aliases {
            project_types.insert(alias.as_str());
            project_types.insert(original.as_str());
        }
    }
    (by_name, project_types)
}

/// For one dispatcher, cluster its called_functions by their normalised
/// signature. Return clusters with ≥ min_size siblings.
fn find_sibling_groups<'a>(
    dispatcher: &FunctionInfo,
    by_name: &HashMap<&str, (&'a PathBuf, &'a FunctionInfo)>,
    min_size: usize,
) -> Vec<Vec<&'a FunctionInfo>> {
    let mut clusters: HashMap<SignatureKey, Vec<&FunctionInfo>> = HashMap::new();
    for name in &dispatcher.called_functions {
        let Some((_, fi)) = by_name.get(name.as_str()) else {
            continue;
        };
        if fi.parameter_types.is_empty() {
            continue;
        }
        let key = signature_key(fi);
        clusters.entry(key).or_default().push(*fi);
    }
    clusters
        .into_values()
        .filter(|v| v.len() >= min_size)
        .collect()
}

type SignatureKey = Vec<String>;

fn signature_key(f: &FunctionInfo) -> SignatureKey {
    f.parameter_types.iter().map(|t| t.name.clone()).collect()
}

/// Check if a group of same-signature siblings shares a non-local type in
/// any parameter position. Emit one finding for the first leaked position.
fn check_param_leak(
    dispatcher_path: &Path,
    dispatcher: &FunctionInfo,
    callbacks: &[&FunctionInfo],
    min_group_size: usize,
    project_types: &HashSet<&str>,
) -> Option<Finding> {
    let arity = callbacks.first()?.parameter_types.len();
    for position in 0..arity {
        let shared = callbacks[0].parameter_types.get(position)?;
        if is_interesting_leak(shared, project_types) {
            return Some(build_abl_finding(
                dispatcher_path,
                dispatcher,
                callbacks,
                shared,
                position,
                min_group_size,
            ));
        }
    }
    None
}

/// Check if a group of same-signature siblings all return the same non-local
/// type — the dual of `check_param_leak`. Handlers leak an external result
/// into the dispatcher's caller instead of translating it.
fn check_return_leak(
    dispatcher_path: &Path,
    dispatcher: &FunctionInfo,
    callbacks: &[&FunctionInfo],
    min_group_size: usize,
    project_types: &HashSet<&str>,
) -> Option<Finding> {
    let first = callbacks.first()?.return_type.as_ref()?;
    // All callbacks must declare the same return type name (the signature
    // cluster didn't include return type, so recheck here).
    if !callbacks.iter().all(|cb| {
        cb.return_type
            .as_ref()
            .is_some_and(|t| t.name == first.name)
    }) {
        return None;
    }
    if !is_interesting_leak(first, project_types) {
        return None;
    }
    Some(build_rtl_finding(
        dispatcher_path,
        dispatcher,
        callbacks,
        first,
        min_group_size,
    ))
}

fn is_interesting_leak(t: &TypeRef, project_types: &HashSet<&str>) -> bool {
    // External types are always leaks. Unknown types may be Local in disguise
    // if they appear in the project's type registry.
    match &t.origin {
        TypeOrigin::External(_) => true,
        TypeOrigin::Unknown => !project_types.contains(t.name.as_str()),
        _ => false,
    }
}

fn build_abl_finding(
    path: &Path,
    dispatcher: &FunctionInfo,
    callbacks: &[&FunctionInfo],
    shared: &TypeRef,
    position: usize,
    min_group_size: usize,
) -> Finding {
    let names: Vec<&str> = callbacks.iter().map(|c| c.name.as_str()).collect();
    let module_hint = module_hint(shared);
    let message = format!(
        "Dispatcher `{}` fans out to {} handlers that all take `{}` {} in position #{} — consider a local DTO (Anti-Corruption Layer)",
        dispatcher.name,
        callbacks.len(),
        shared.name,
        module_hint,
        position + 1,
    );
    Finding {
        smell_name: ABL_SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: dispatcher_location(path, dispatcher),
        message,
        suggested_refactorings: vec![
            format!(
                "Introduce `{}Data` DTO in the project and convert `{}` before calling handlers",
                pascal_case(&shared.name),
                shared.name
            ),
            "Move the conversion step into the dispatcher (Anti-Corruption Layer pattern)".into(),
            rename_suggestion(&names),
        ],
        actual_value: Some(callbacks.len() as f64),
        threshold: Some(min_group_size as f64),
        risk_score: None,
    }
}

fn build_rtl_finding(
    path: &Path,
    dispatcher: &FunctionInfo,
    callbacks: &[&FunctionInfo],
    shared: &TypeRef,
    min_group_size: usize,
) -> Finding {
    let module_hint = module_hint(shared);
    let message = format!(
        "Dispatcher `{}` fans out to {} handlers that all return `{}` {} — the external type escapes to callers; consider returning a local DTO",
        dispatcher.name,
        callbacks.len(),
        shared.name,
        module_hint,
    );
    Finding {
        smell_name: RTL_SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: dispatcher_location(path, dispatcher),
        message,
        suggested_refactorings: vec![
            format!(
                "Define a local `{}Result` DTO, return it from each handler, convert to `{}` once at the outer edge",
                pascal_case(&shared.name),
                shared.name
            ),
            "Centralise the external-type handling in one adapter instead of each handler producing it".into(),
        ],
        actual_value: Some(callbacks.len() as f64),
        threshold: Some(min_group_size as f64),
        risk_score: None,
    }
}

fn module_hint(shared: &TypeRef) -> String {
    match &shared.origin {
        TypeOrigin::External(m) => format!("from `{m}`"),
        TypeOrigin::Unknown => "(origin unresolved)".to_string(),
        _ => String::new(),
    }
}

fn dispatcher_location(path: &Path, dispatcher: &FunctionInfo) -> Location {
    Location {
        path: path.to_path_buf(),
        start_line: dispatcher.start_line,
        start_col: dispatcher.name_col,
        end_line: dispatcher.start_line,
        end_col: dispatcher.name_end_col,
        name: Some(dispatcher.name.clone()),
    }
}

fn pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(capitalise)
        .collect()
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn rename_suggestion(names: &[&str]) -> String {
    if has_common_prefix(names) {
        "Group already uses a shared naming convention".into()
    } else {
        "Consider renaming handlers with a shared prefix (`on_*`, `handle_*`, `visit_*`)".into()
    }
}

fn has_common_prefix(names: &[&str]) -> bool {
    let prefixes = [
        "on_",
        "handle_",
        "visit_",
        "check_",
        "render_",
        "transform_",
    ];
    let matching = names
        .iter()
        .filter(|n| prefixes.iter().any(|p| n.starts_with(p)))
        .count();
    matching * 100 >= names.len() * 60
}

#[cfg(test)]
mod tests;
