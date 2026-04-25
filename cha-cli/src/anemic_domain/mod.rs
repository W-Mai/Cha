//! Detect the Anemic Domain Model anti-pattern: a class that is pure data
//! (fields, no behavior) paired with a service/manager function elsewhere that
//! owns the behavior that "belongs" on the class.
//!
//! Distinct from `data_class` (a Dispensables hint) — this needs cross-file
//! evidence of the paired service to promote a data-only class into an actual
//! architectural smell, so it lives as a post-analysis pass.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cha_core::{ClassInfo, Finding, FunctionInfo, Location, Severity, SmellCategory};

const SMELL: &str = "anemic_domain_model";
const MIN_FIELDS: usize = 2;

/// Run the detector across all project models.
pub fn detect(
    files: &[PathBuf],
    cwd: &Path,
    cache: &std::sync::Mutex<cha_core::ProjectCache>,
) -> Vec<Finding> {
    let models: Vec<(PathBuf, cha_core::SourceModel)> = files
        .iter()
        .filter_map(|p| {
            let mut c = cache.lock().ok()?;
            let (_, model) = crate::cached_parse(p, &mut c, cwd)?;
            Some((p.clone(), model))
        })
        .collect();
    detect_from_models(&models)
}

fn detect_from_models(models: &[(PathBuf, cha_core::SourceModel)]) -> Vec<Finding> {
    let services_by_target = build_service_index(models);
    let mut findings = Vec::new();
    for (path, model) in models {
        for class in &model.classes {
            if !is_anemic(class) {
                continue;
            }
            let Some(services) = services_by_target.get(class.name.as_str()) else {
                continue;
            };
            // A function that lives in the same file as the class itself is
            // treated as part of the class (e.g. a Rust `impl` block next to
            // the struct, a Python class file's top-level helper) — not an
            // external service.
            let external: Vec<(&Path, &FunctionInfo)> = services
                .iter()
                .filter(|(p, _)| *p != path.as_path())
                .copied()
                .collect();
            if external.is_empty() {
                continue;
            }
            findings.push(build_finding(path, class, &external));
        }
    }
    findings
}

fn is_anemic(c: &ClassInfo) -> bool {
    !c.is_interface && !c.has_behavior && c.field_count >= MIN_FIELDS
}

/// Index: type name → list of (path, function) pairs where the function is
/// service-shaped and takes that type as its first parameter.
type ServiceIndex<'a> = HashMap<&'a str, Vec<(&'a Path, &'a FunctionInfo)>>;

fn build_service_index<'a>(models: &'a [(PathBuf, cha_core::SourceModel)]) -> ServiceIndex<'a> {
    let mut index: ServiceIndex<'a> = HashMap::new();
    for (path, model) in models {
        for f in &model.functions {
            let Some(first_param) = f.parameter_types.first() else {
                continue;
            };
            if !is_service_shaped(&f.name, path) {
                continue;
            }
            index
                .entry(first_param.name.as_str())
                .or_default()
                .push((path.as_path(), f));
        }
    }
    index
}

/// A function looks "service-shaped" if its name (or enclosing file) signals
/// behavior that arguably belongs on the data class. Kept conservative — the
/// strongest signal is PascalCase suffixes like `FooService`, `FooManager` on
/// the enclosing filename; secondary signal is verb-prefixed function names.
fn is_service_shaped(fn_name: &str, path: &Path) -> bool {
    has_service_verb_prefix(fn_name) || file_is_service(path)
}

fn has_service_verb_prefix(fn_name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "process_",
        "validate_",
        "handle_",
        "serialize_",
        "deserialize_",
        "calculate_",
        "compute_",
        "transform_",
        "convert_",
        "apply_",
        "update_",
    ];
    PREFIXES.iter().any(|p| fn_name.starts_with(p))
}

fn file_is_service(path: &Path) -> bool {
    const SUFFIXES: &[&str] = &["service", "manager", "handler", "helper", "util", "utils"];
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let lower = stem.to_ascii_lowercase();
    SUFFIXES.iter().any(|s| lower.ends_with(s))
}

fn build_finding(path: &Path, class: &ClassInfo, services: &[(&Path, &FunctionInfo)]) -> Finding {
    let service_names: Vec<&str> = services.iter().map(|(_, f)| f.name.as_str()).collect();
    let shown = service_names.iter().take(3).copied().collect::<Vec<_>>();
    let suffix = if service_names.len() > 3 {
        format!(" (+{} more)", service_names.len() - 3)
    } else {
        String::new()
    };
    let message = format!(
        "Class `{}` has {} fields and no behavior, but `{}`{} operate on it — the class is anemic; consider moving behavior onto it",
        class.name,
        class.field_count,
        shown.join("`, `"),
        suffix,
    );
    Finding {
        smell_name: SMELL.into(),
        category: SmellCategory::OoAbusers,
        severity: Severity::Hint,
        location: Location {
            path: path.to_path_buf(),
            start_line: class.start_line,
            start_col: class.name_col,
            end_line: class.start_line,
            end_col: class.name_end_col,
            name: Some(class.name.clone()),
        },
        message,
        suggested_refactorings: vec![
            format!(
                "Move behavior from service functions onto `{}` (Move Method)",
                class.name
            ),
            "Replace Data Class with a proper domain object that owns its invariants".into(),
        ],
        actual_value: Some(services.len() as f64),
        threshold: Some(1.0),
    }
}

#[cfg(test)]
mod tests;
