//! Cross-layer import: automatically infer project layers from the import
//! graph, then flag imports that go from a higher (more stable) layer to a
//! lower (less stable) one. Same idea as the configured `layer_violation`
//! plugin, but zero configuration — runs on every project.
//!
//! Layer inference uses Martin's instability (`fan_out / (fan_in + fan_out)`)
//! as a ranking signal. A `domain`-like module with zero fan-out and many
//! fan-in lands at instability 0 (most stable); an `application`/`ui`
//! module at the top. Violations are edges that go upward (stable →
//! unstable) — those are the imports a clean architecture forbids.

use std::path::{Path, PathBuf};

use cha_core::{Finding, Location, Severity, SmellCategory, graph};

const SMELL: &str = "cross_layer_import";
const MIN_GAP: f64 = 0.3;

pub fn detect(
    files: &[PathBuf],
    cwd: &Path,
    cache: &std::sync::Mutex<cha_core::ProjectCache>,
) -> Vec<Finding> {
    // If a manual layer config exists, defer to `layer_violation` plugin —
    // no point producing two competing findings for the same idea.
    let config = crate::load_config(cwd);
    if !config.layers.modules.is_empty() && !config.layers.tiers.is_empty() {
        return Vec::new();
    }

    let mut cache_guard = match cache.lock() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let (file_imports, all_files) = crate::layers::build_import_edges(files, cwd, &mut cache_guard);
    drop(cache_guard);

    let modules = graph::infer_modules(&file_imports, &all_files, None);
    if modules.len() < 2 {
        return Vec::new();
    }
    let (_layers, violations) = graph::infer_layers(&modules, &file_imports);
    violations
        .into_iter()
        .filter(|v| v.gap >= MIN_GAP)
        .flat_map(|v| violation_to_findings(&v, cwd))
        .collect()
}

fn violation_to_findings(v: &graph::LayerViolation, cwd: &Path) -> Vec<Finding> {
    v.evidence
        .iter()
        .map(|(from_file, to_file)| Finding {
            smell_name: SMELL.into(),
            category: SmellCategory::Couplers,
            severity: Severity::Warning,
            location: Location {
                path: cwd.join(from_file),
                start_line: 1,
                end_line: 1,
                ..Default::default()
            },
            message: format!(
                "`{}` (inferred layer `{}`) imports from `{}` (inferred layer `{}`) — crosses a layer boundary upward (instability gap {:.2})",
                from_file, v.from_module, to_file, v.to_module, v.gap,
            ),
            suggested_refactorings: vec![
                format!("Move the shared concept into a lower-layer module both sides can depend on"),
                format!("Invert the dependency: have `{}` expose a trait that `{}` implements", v.from_module, v.to_module),
            ],
            actual_value: Some(v.gap),
            threshold: Some(MIN_GAP),
        })
        .collect()
}

#[cfg(test)]
mod tests;
