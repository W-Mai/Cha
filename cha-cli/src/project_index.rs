//! `ProjectIndex` — parse every file once and pre-compute the derived indices
//! the signature-based post-analysis passes need. Before this, each pass
//! (`boundary_leak`, `anemic_domain`, `typed_intimacy`, `module_envy`) built
//! its own copies of the same maps on every run.
//!
//! Passes take a `&ProjectIndex` instead of `(files, cwd, cache)`. They still
//! do their own per-finding work; only the up-front parse and index steps are
//! shared.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::{
    ClassInfo, FunctionInfo, ProjectQuery, ProjectQueryBulk, SourceModel, TypeOrigin, TypeRef,
};

/// Everything the signature-based detectors need to do their work.
// cha:ignore large_class
pub struct ProjectIndex {
    models: Vec<(PathBuf, SourceModel)>,
    function_home: HashMap<String, PathBuf>,
    class_home: HashMap<String, PathBuf>,
    function_by_name: HashMap<String, usize>,
    project_type_names: HashSet<String>,
    // Precomputed for ProjectQuery trait.
    callers_index: HashMap<String, Vec<PathBuf>>,
    cross_file_calls: Vec<((PathBuf, PathBuf), u32)>,
    workspace_crates: HashSet<String>,
    model_index: HashMap<PathBuf, usize>,
}

impl ProjectIndex {
    /// Build from a parsed set of models. Freshness of the models is the
    /// caller's concern.
    pub fn from_models(models: Vec<(PathBuf, SourceModel)>) -> Self {
        let function_home = build_function_home(&models);
        let class_home = build_class_home(&models);
        let function_by_name = build_function_by_name(&models);
        let project_type_names = build_project_type_names(&models);
        let callers_index = build_callers_index(&models);
        let cross_file_calls = build_cross_file_calls(&models, &function_home);
        let workspace_crates = build_workspace_crates(&models);
        let model_index = build_model_index(&models);
        Self {
            models,
            function_home,
            class_home,
            function_by_name,
            project_type_names,
            callers_index,
            cross_file_calls,
            workspace_crates,
            model_index,
        }
    }

    /// Parse every file via the shared cache and assemble an index. Used by
    /// the real cha-cli pipeline; tests build their own models and call
    /// `from_models` directly.
    pub fn parse(
        files: &[PathBuf],
        cwd: &Path,
        cache: &std::sync::Mutex<cha_core::ProjectCache>,
    ) -> Self {
        let mut models: Vec<(PathBuf, SourceModel)> = files
            .iter()
            .filter_map(|p| {
                let mut c = cache.lock().ok()?;
                let (_, model) = crate::cached_parse(p, &mut c, cwd)?;
                Some((p.clone(), model))
            })
            .collect();
        // Post-parse enrichment: for C / C++ projects, rewrite ClassInfo
        // method counts, has_behavior, and is_exported so downstream
        // detectors see the correct OOP shape. No-op for non-C projects.
        crate::c_oop_enrich::enrich_c_oop(&mut models);
        Self::from_models(models)
    }

    pub fn models(&self) -> &[(PathBuf, SourceModel)] {
        &self.models
    }

    /// Where a function name was first declared across the project.
    pub fn function_home(&self) -> &HashMap<String, PathBuf> {
        &self.function_home
    }

    /// Where a class/struct name was first declared across the project.
    pub fn class_home(&self) -> &HashMap<String, PathBuf> {
        &self.class_home
    }

    /// Find a function by name — returns `(path, info)` of the first decl.
    /// Used by future passes (e.g. parameter_position_inconsistency).
    #[allow(dead_code)]
    pub fn lookup_function(&self, name: &str) -> Option<(&Path, &FunctionInfo)> {
        let idx = *self.function_by_name.get(name)?;
        let (path, model) = &self.models[idx];
        model
            .functions
            .iter()
            .find(|f| f.name == name)
            .map(|f| (path.as_path(), f))
    }

    /// All type names known in the project — classes, typedef aliases, and
    /// the "original" side of each alias. Used for Unknown-origin fallback.
    #[allow(dead_code)]
    pub fn project_type_names(&self) -> &HashSet<String> {
        &self.project_type_names
    }

    /// Every class in the project, paired with the file it lives in.
    #[allow(dead_code)]
    pub fn all_classes(&self) -> impl Iterator<Item = (&Path, &ClassInfo)> {
        self.models
            .iter()
            .flat_map(|(p, m)| m.classes.iter().map(move |c| (p.as_path(), c)))
    }
}

impl ProjectQuery for ProjectIndex {
    fn is_called_externally(&self, name: &str, exclude_path: &Path) -> bool {
        self.callers_index
            .get(name)
            .is_some_and(|callers| callers.iter().any(|p| p.as_path() != exclude_path))
    }

    fn callers_of(&self, name: &str) -> Vec<PathBuf> {
        self.callers_index.get(name).cloned().unwrap_or_default()
    }

    fn cross_file_call_counts(&self) -> Vec<((PathBuf, PathBuf), u32)> {
        self.cross_file_calls.clone()
    }

    fn function_home(&self, name: &str) -> Option<PathBuf> {
        self.function_home.get(name).cloned()
    }

    fn function_by_name(&self, name: &str) -> Option<(PathBuf, FunctionInfo)> {
        let idx = *self.function_by_name.get(name)?;
        let (path, model) = &self.models[idx];
        model
            .functions
            .iter()
            .find(|f| f.name == name)
            .map(|f| (path.clone(), f.clone()))
    }

    fn class_home(&self, name: &str) -> Option<PathBuf> {
        self.class_home.get(name).cloned()
    }

    fn model_by_path(&self, path: &Path) -> Option<SourceModel> {
        let idx = *self.model_index.get(path)?;
        Some(self.models[idx].1.clone())
    }

    fn is_project_type(&self, name: &str) -> bool {
        self.project_type_names.contains(name)
    }

    fn is_third_party(&self, type_ref: &TypeRef) -> bool {
        match &type_ref.origin {
            TypeOrigin::External(crate_name) => {
                !is_stdlib_crate(crate_name) && !self.workspace_crates.contains(crate_name)
            }
            _ => false,
        }
    }

    fn workspace_crate_names(&self) -> Vec<String> {
        self.workspace_crates.iter().cloned().collect()
    }

    fn is_test_path(&self, path: &Path) -> bool {
        is_test_path_impl(path)
    }

    fn file_count(&self) -> usize {
        self.models.len()
    }
}

impl ProjectQueryBulk for ProjectIndex {
    fn iter_models(&self) -> Box<dyn Iterator<Item = (&Path, &SourceModel)> + '_> {
        Box::new(self.models.iter().map(|(p, m)| (p.as_path(), m)))
    }
}

/// Stdlib / standard crate names that are never "third-party" leaks.
/// Mirrors what leaky_public::is_external_leak considered stdlib.
fn is_stdlib_crate(name: &str) -> bool {
    matches!(name, "std" | "core" | "alloc" | "test" | "proc_macro")
}

/// Shared "is this path under a test directory?" predicate.
/// Used to live as private fns in boundary_leak and module_envy with
/// identical implementations.
fn is_test_path_impl(path: &Path) -> bool {
    const TEST_DIRS: &[&str] = &["test", "tests", "__tests__", "__mocks__"];
    let lossy = path.to_string_lossy();
    if TEST_DIRS
        .iter()
        .any(|d| lossy.contains(&format!("/{}/", d)) || lossy.contains(&format!("/{}\\", d)))
    {
        return true;
    }
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        && (stem.ends_with("_test") || stem.starts_with("test_"))
    {
        return true;
    }
    false
}

fn build_function_home(models: &[(PathBuf, SourceModel)]) -> HashMap<String, PathBuf> {
    let mut home: HashMap<String, PathBuf> = HashMap::new();
    for (path, model) in models {
        for f in &model.functions {
            home.entry(f.name.clone()).or_insert_with(|| path.clone());
        }
    }
    home
}

fn build_class_home(models: &[(PathBuf, SourceModel)]) -> HashMap<String, PathBuf> {
    let mut home: HashMap<String, PathBuf> = HashMap::new();
    for (path, model) in models {
        for c in &model.classes {
            home.entry(c.name.clone()).or_insert_with(|| path.clone());
        }
    }
    home
}

fn build_function_by_name(models: &[(PathBuf, SourceModel)]) -> HashMap<String, usize> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for (idx, (_, model)) in models.iter().enumerate() {
        for f in &model.functions {
            map.entry(f.name.clone()).or_insert(idx);
        }
    }
    map
}

fn build_project_type_names(models: &[(PathBuf, SourceModel)]) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    for (_, model) in models {
        for c in &model.classes {
            names.insert(c.name.clone());
        }
        for (alias, original) in &model.type_aliases {
            names.insert(alias.clone());
            names.insert(original.clone());
        }
    }
    names
}

fn build_callers_index(models: &[(PathBuf, SourceModel)]) -> HashMap<String, Vec<PathBuf>> {
    let mut index: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for (path, model) in models {
        for f in &model.functions {
            for callee in &f.called_functions {
                let entry = index.entry(callee.clone()).or_default();
                if !entry.iter().any(|p| p == path) {
                    entry.push(path.clone());
                }
            }
        }
    }
    index
}

fn build_cross_file_calls(
    models: &[(PathBuf, SourceModel)],
    function_home: &HashMap<String, PathBuf>,
) -> Vec<((PathBuf, PathBuf), u32)> {
    let mut counts: HashMap<(PathBuf, PathBuf), u32> = HashMap::new();
    for (caller_path, model) in models {
        for f in &model.functions {
            for callee in &f.called_functions {
                if let Some(callee_path) = function_home.get(callee)
                    && callee_path != caller_path
                {
                    *counts
                        .entry((caller_path.clone(), callee_path.clone()))
                        .or_default() += 1;
                }
            }
        }
    }
    counts.into_iter().collect()
}

fn build_workspace_crates(models: &[(PathBuf, SourceModel)]) -> HashSet<String> {
    let mut crates = HashSet::new();
    for (path, _) in models {
        if let Some(first) = path.iter().next()
            && let Some(s) = first.to_str()
        {
            crates.insert(s.to_string());
        }
    }
    crates
}

fn build_model_index(models: &[(PathBuf, SourceModel)]) -> HashMap<PathBuf, usize> {
    models
        .iter()
        .enumerate()
        .map(|(i, (p, _))| (p.clone(), i))
        .collect()
}
