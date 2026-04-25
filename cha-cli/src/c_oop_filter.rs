//! Post-analysis filter that removes `lazy_class` / `data_class` false
//! positives on C structs that have cross-file "methods" — functions in the
//! same directory whose first parameter is a pointer to the struct (or its
//! typedef alias). Those structs are de-facto OOP classes in C even though
//! tree-sitter can't attach the methods to them lexically.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::Finding;

pub fn filter_c_oop_false_positives(
    findings: Vec<Finding>,
    files: &[PathBuf],
    cache: &std::sync::Mutex<cha_core::ProjectCache>,
    cwd: &Path,
) -> Vec<Finding> {
    // Skip expensive C OOP analysis if no lazy_class/data_class findings exist
    if !findings
        .iter()
        .any(|f| matches!(f.smell_name.as_str(), "lazy_class" | "data_class"))
    {
        return findings;
    }
    let has_methods = collect_c_structs_with_methods(files, cache, cwd);
    if has_methods.is_empty() {
        return findings;
    }
    findings
        .into_iter()
        .filter(|f| {
            if !matches!(f.smell_name.as_str(), "lazy_class" | "data_class") {
                return true;
            }
            let name = f.location.name.as_deref().unwrap_or("");
            !has_methods.contains(name)
        })
        .collect()
}

/// Parse C files and find structs that have cross-file methods.
fn collect_c_structs_with_methods(
    files: &[PathBuf],
    cache: &std::sync::Mutex<cha_core::ProjectCache>,
    cwd: &Path,
) -> HashSet<String> {
    let models: Vec<(PathBuf, cha_core::SourceModel)> = files
        .iter()
        .filter(|f| matches!(f.extension().and_then(|e| e.to_str()), Some("c" | "h")))
        .filter_map(|p| {
            let mut c = cache.lock().ok()?;
            let (_, model) = crate::cached_parse(p, &mut c, cwd)?;
            Some((p.clone(), model))
        })
        .collect();
    if models.is_empty() {
        return HashSet::new();
    }

    let mut aliases: HashMap<String, String> = HashMap::new();
    for (_, m) in &models {
        for (a, o) in &m.type_aliases {
            aliases.entry(a.clone()).or_insert(o.clone());
        }
    }
    let reverse: HashMap<&str, &str> = aliases
        .iter()
        .map(|(a, o)| (o.as_str(), a.as_str()))
        .collect();

    let mut dir_funcs: HashMap<&Path, Vec<&cha_core::FunctionInfo>> = HashMap::new();
    for (path, m) in &models {
        let dir = path.parent().unwrap_or(path);
        dir_funcs.entry(dir).or_default().extend(&m.functions);
    }

    let mut result = HashSet::new();
    for (path, m) in &models {
        let dir = path.parent().unwrap_or(path);
        let funcs = dir_funcs.get(dir).cloned().unwrap_or_default();
        for c in &m.classes {
            let alias = reverse.get(c.name.as_str()).copied().unwrap_or(&c.name);
            if has_pointer_param_method(&funcs, &c.name, alias) {
                result.insert(c.name.clone());
                result.insert(alias.to_string());
            }
        }
    }
    result
}

/// Check if any function's first parameter is a pointer to the given struct.
fn has_pointer_param_method(funcs: &[&cha_core::FunctionInfo], name: &str, alias: &str) -> bool {
    funcs.iter().any(|f| {
        f.parameter_types.first().is_some_and(|t| {
            t.raw.contains('*') && {
                let base = t.raw.split('*').next().unwrap_or("").trim();
                base == name || base == alias
            }
        })
    })
}
