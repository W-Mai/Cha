//! Abstraction Boundary Leak (ABL-0) detector.
//!
//! Flags dispatcher functions that fan out to ≥ N sibling callbacks which
//! all share the same non-local type in corresponding parameter positions.
//! The fix is to introduce a local DTO and have the dispatcher translate
//! the external type into it — the Anti-Corruption Layer pattern.
//!
//! Implementation plan lives in `.kiro/specs/abstraction-boundary-leak/`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory, TypeOrigin, TypeRef};

const SMELL_NAME: &str = "abstraction_boundary_leak";
const DEFAULT_MIN_GROUP_SIZE: usize = 3;

/// Run the detector across all project models. Returns hint-severity findings.
pub fn detect(
    files: &[PathBuf],
    _cwd: &Path,
    _cache: &std::sync::Mutex<cha_core::ProjectCache>,
) -> Vec<Finding> {
    // Parse every file fresh. The boundary-leak detector is sensitive to
    // every typedef alias in the project — going through the shared cache
    // has occasionally surfaced models with fewer aliases than a fresh parse
    // (root cause TBD), and the extra parse pass is already amortised with
    // the main analyze phase's per-file work, so we accept the cost.
    let models: Vec<(PathBuf, cha_core::SourceModel)> = files
        .iter()
        .filter_map(|p| {
            let content = std::fs::read_to_string(p).ok()?;
            let file = cha_core::SourceFile::new(p.clone(), content);
            cha_parser::parse_file(&file).map(|m| (p.clone(), m))
        })
        .collect();
    detect_from_models(&models, DEFAULT_MIN_GROUP_SIZE)
}

fn detect_from_models(
    models: &[(PathBuf, cha_core::SourceModel)],
    min_group_size: usize,
) -> Vec<Finding> {
    let (by_name, project_types) = build_indices(models);
    let mut findings = Vec::new();
    for (path, model) in models {
        for dispatcher in &model.functions {
            append_dispatcher_findings(
                path,
                dispatcher,
                &by_name,
                &project_types,
                min_group_size,
                &mut findings,
            );
        }
    }
    findings
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
        if let Some(finding) = check_group(path, dispatcher, &group, min_group_size, project_types)
        {
            findings.push(finding);
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
/// any parameter position. Emit one finding per leaked position.
fn check_group(
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
            return Some(build_finding(
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

fn is_interesting_leak(t: &TypeRef, project_types: &HashSet<&str>) -> bool {
    // External types are always leaks. Unknown types may be Local in disguise
    // if they appear in the project's type registry.
    match &t.origin {
        TypeOrigin::External(_) => true,
        TypeOrigin::Unknown => !project_types.contains(t.name.as_str()),
        _ => false,
    }
}

fn build_finding(
    path: &Path,
    dispatcher: &FunctionInfo,
    callbacks: &[&FunctionInfo],
    shared: &TypeRef,
    position: usize,
    min_group_size: usize,
) -> Finding {
    let names: Vec<&str> = callbacks.iter().map(|c| c.name.as_str()).collect();
    let module_hint = match &shared.origin {
        TypeOrigin::External(m) => format!("from `{m}`"),
        TypeOrigin::Unknown => "(origin unresolved)".to_string(),
        _ => String::new(),
    };
    let message = format!(
        "Dispatcher `{}` fans out to {} handlers that all take `{}` {} in position #{} — consider a local DTO (Anti-Corruption Layer)",
        dispatcher.name,
        callbacks.len(),
        shared.name,
        module_hint,
        position + 1,
    );
    Finding {
        smell_name: SMELL_NAME.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: Location {
            path: path.to_path_buf(),
            start_line: dispatcher.start_line,
            start_col: dispatcher.name_col,
            end_line: dispatcher.start_line,
            end_col: dispatcher.name_end_col,
            name: Some(dispatcher.name.clone()),
        },
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
mod tests {
    use super::*;
    use cha_core::SourceModel;

    fn func(name: &str, params: Vec<TypeRef>, calls: Vec<String>) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            start_line: 1,
            end_line: 1,
            parameter_count: params.len(),
            parameter_types: params,
            called_functions: calls,
            ..Default::default()
        }
    }

    fn tref(name: &str, origin: TypeOrigin) -> TypeRef {
        TypeRef {
            name: name.into(),
            raw: name.into(),
            origin,
        }
    }

    fn model(functions: Vec<FunctionInfo>) -> SourceModel {
        SourceModel {
            language: "rust".into(),
            total_lines: 10,
            functions,
            classes: vec![],
            imports: vec![],
            comments: vec![],
            type_aliases: vec![],
        }
    }

    #[test]
    fn flags_external_type_in_callback_group() {
        let ts = tref("Node", TypeOrigin::External("tree_sitter".into()));
        let callbacks = vec![
            func("on_a", vec![ts.clone()], vec![]),
            func("on_b", vec![ts.clone()], vec![]),
            func("on_c", vec![ts.clone()], vec![]),
        ];
        let dispatcher = func(
            "dispatch",
            vec![],
            vec!["on_a".into(), "on_b".into(), "on_c".into()],
        );
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "abstraction_boundary_leak");
        assert!(
            findings[0].message.contains("tree_sitter"),
            "message should cite the external module: {}",
            findings[0].message
        );
    }

    #[test]
    fn ignores_group_of_local_types() {
        let local = tref("Finding", TypeOrigin::Local);
        let callbacks = vec![
            func("check_a", vec![local.clone()], vec![]),
            func("check_b", vec![local.clone()], vec![]),
            func("check_c", vec![local.clone()], vec![]),
        ];
        let dispatcher = func(
            "analyze",
            vec![],
            vec!["check_a".into(), "check_b".into(), "check_c".into()],
        );
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        assert!(findings.is_empty(), "local types should not trigger");
    }

    #[test]
    fn ignores_divergent_signatures() {
        // 3 siblings but each with different signature — no group reaches min_size.
        let dispatcher = func("dispatch", vec![], vec!["a".into(), "b".into(), "c".into()]);
        let ts = |n| tref("Node", TypeOrigin::External("tree_sitter".into()));
        let callbacks = vec![
            func("a", vec![ts("a")], vec![]),
            func(
                "b",
                vec![
                    tref("Node", TypeOrigin::External("tree_sitter".into())),
                    tref("Node", TypeOrigin::External("tree_sitter".into())),
                ],
                vec![],
            ),
            func("c", vec![], vec![]),
        ];
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_below_threshold() {
        let ts = tref("Node", TypeOrigin::External("tree_sitter".into()));
        let callbacks = vec![
            func("on_a", vec![ts.clone()], vec![]),
            func("on_b", vec![ts.clone()], vec![]),
        ];
        let dispatcher = func("dispatch", vec![], vec!["on_a".into(), "on_b".into()]);
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        assert!(findings.is_empty(), "2 callbacks < min_group_size 3");
    }

    #[test]
    fn unknown_origin_reported_with_qualifier() {
        let u = tref("cmark_node_t", TypeOrigin::Unknown);
        let callbacks = vec![
            func("on_a", vec![u.clone()], vec![]),
            func("on_b", vec![u.clone()], vec![]),
            func("on_c", vec![u.clone()], vec![]),
        ];
        let dispatcher = func(
            "dispatch",
            vec![],
            vec!["on_a".into(), "on_b".into(), "on_c".into()],
        );
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.c"), model(all))];
        let findings = detect_from_models(&models, 3);
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].message.contains("unresolved"),
            "unknown origin should include lower-confidence qualifier: {}",
            findings[0].message
        );
    }

    #[test]
    fn rename_suggestion_skipped_when_prefix_uniform() {
        let ts = tref("Node", TypeOrigin::External("tree_sitter".into()));
        let callbacks = vec![
            func("on_a", vec![ts.clone()], vec![]),
            func("on_b", vec![ts.clone()], vec![]),
            func("on_c", vec![ts.clone()], vec![]),
        ];
        let dispatcher = func(
            "dispatch",
            vec![],
            vec!["on_a".into(), "on_b".into(), "on_c".into()],
        );
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        let last = findings[0].suggested_refactorings.last().unwrap();
        assert!(
            last.contains("already uses"),
            "expected shared-convention hint, got: {last}"
        );
    }
}
