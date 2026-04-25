//! Signature-based abstraction leak detectors.
//!
//! Two smells share a detector pipeline here:
//!
//! - `abstraction_boundary_leak` — dispatcher fans out to ≥ N sibling
//!   callbacks that all take the same non-local type in corresponding
//!   parameter positions. Missing Anti-Corruption Layer on the way *in*.
//! - `return_type_leak` — dispatcher fans out to ≥ N sibling callbacks
//!   whose **return types** are all the same non-local type. Missing
//!   Anti-Corruption Layer on the way *out*. The fix is usually a local
//!   DTO returned from the handlers plus one place that converts to the
//!   external type, rather than every handler touching it directly.
//!
//! Both share sibling-clustering by signature + the project-wide type
//! registry (`cha-parser` surfaces TypeOrigin on each parameter and on
//! the return type).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory, TypeOrigin, TypeRef};

const ABL_SMELL: &str = "abstraction_boundary_leak";
const RTL_SMELL: &str = "return_type_leak";
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

    // --- return_type_leak ---

    fn func_with_return(
        name: &str,
        params: Vec<TypeRef>,
        calls: Vec<String>,
        ret: Option<TypeRef>,
    ) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            start_line: 1,
            end_line: 1,
            parameter_count: params.len(),
            parameter_types: params,
            called_functions: calls,
            return_type: ret,
            ..Default::default()
        }
    }

    #[test]
    fn flags_external_return_type_in_callback_group() {
        let param = tref("Ctx", TypeOrigin::Local);
        let external_ret = tref("Value", TypeOrigin::External("serde_json".into()));
        let callbacks = vec![
            func_with_return(
                "on_a",
                vec![param.clone()],
                vec![],
                Some(external_ret.clone()),
            ),
            func_with_return(
                "on_b",
                vec![param.clone()],
                vec![],
                Some(external_ret.clone()),
            ),
            func_with_return(
                "on_c",
                vec![param.clone()],
                vec![],
                Some(external_ret.clone()),
            ),
        ];
        let dispatcher = func_with_return(
            "dispatch",
            vec![],
            vec!["on_a".into(), "on_b".into(), "on_c".into()],
            None,
        );
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        let rtl: Vec<_> = findings
            .iter()
            .filter(|f| f.smell_name == "return_type_leak")
            .collect();
        assert_eq!(rtl.len(), 1, "expected one return_type_leak finding");
        assert!(
            rtl[0].message.contains("serde_json"),
            "msg: {}",
            rtl[0].message
        );
    }

    #[test]
    fn ignores_local_return_type() {
        let param = tref("Ctx", TypeOrigin::Local);
        let local_ret = tref("Finding", TypeOrigin::Local);
        let callbacks = vec![
            func_with_return("on_a", vec![param.clone()], vec![], Some(local_ret.clone())),
            func_with_return("on_b", vec![param.clone()], vec![], Some(local_ret.clone())),
            func_with_return("on_c", vec![param.clone()], vec![], Some(local_ret.clone())),
        ];
        let dispatcher = func_with_return(
            "dispatch",
            vec![],
            vec!["on_a".into(), "on_b".into(), "on_c".into()],
            None,
        );
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        assert!(
            findings.iter().all(|f| f.smell_name != "return_type_leak"),
            "local return types should not trigger"
        );
    }

    #[test]
    fn ignores_divergent_return_types() {
        let param = tref("Ctx", TypeOrigin::Local);
        let ra = tref("A", TypeOrigin::External("ext".into()));
        let rb = tref("B", TypeOrigin::External("ext".into()));
        let callbacks = vec![
            func_with_return("on_a", vec![param.clone()], vec![], Some(ra.clone())),
            func_with_return("on_b", vec![param.clone()], vec![], Some(rb.clone())),
            func_with_return("on_c", vec![param.clone()], vec![], Some(ra.clone())),
        ];
        let dispatcher = func_with_return(
            "dispatch",
            vec![],
            vec!["on_a".into(), "on_b".into(), "on_c".into()],
            None,
        );
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        assert!(
            findings.iter().all(|f| f.smell_name != "return_type_leak"),
            "handlers return different external types, not a unified leak"
        );
    }

    #[test]
    fn return_type_leak_independent_from_param_leak() {
        let local_param = tref("Ctx", TypeOrigin::Local);
        let external_ret = tref("Value", TypeOrigin::External("serde_json".into()));
        let callbacks = vec![
            func_with_return(
                "h_a",
                vec![local_param.clone()],
                vec![],
                Some(external_ret.clone()),
            ),
            func_with_return(
                "h_b",
                vec![local_param.clone()],
                vec![],
                Some(external_ret.clone()),
            ),
            func_with_return(
                "h_c",
                vec![local_param.clone()],
                vec![],
                Some(external_ret.clone()),
            ),
        ];
        let dispatcher = func_with_return(
            "run",
            vec![],
            vec!["h_a".into(), "h_b".into(), "h_c".into()],
            None,
        );
        let mut all = callbacks;
        all.push(dispatcher);
        let models = vec![(PathBuf::from("test.rs"), model(all))];
        let findings = detect_from_models(&models, 3);
        // Param leak should NOT fire (local Ctx); only RTL fires.
        let abl_count = findings
            .iter()
            .filter(|f| f.smell_name == "abstraction_boundary_leak")
            .count();
        let rtl_count = findings
            .iter()
            .filter(|f| f.smell_name == "return_type_leak")
            .count();
        assert_eq!(abl_count, 0);
        assert_eq!(rtl_count, 1);
    }
}
