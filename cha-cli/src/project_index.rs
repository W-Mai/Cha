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

use cha_core::{ClassInfo, FunctionInfo, SourceModel};

/// Everything the signature-based detectors need to do their work.
pub struct ProjectIndex {
    models: Vec<(PathBuf, SourceModel)>,
    function_home: HashMap<String, PathBuf>,
    class_home: HashMap<String, PathBuf>,
    function_by_name: HashMap<String, usize>,
    project_type_names: HashSet<String>,
}

impl ProjectIndex {
    /// Build from a parsed set of models. Freshness of the models is the
    /// caller's concern.
    pub fn from_models(models: Vec<(PathBuf, SourceModel)>) -> Self {
        let function_home = build_function_home(&models);
        let class_home = build_class_home(&models);
        let function_by_name = build_function_by_name(&models);
        let project_type_names = build_project_type_names(&models);
        Self {
            models,
            function_home,
            class_home,
            function_by_name,
            project_type_names,
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
        let models: Vec<(PathBuf, SourceModel)> = files
            .iter()
            .filter_map(|p| {
                let mut c = cache.lock().ok()?;
                let (_, model) = crate::cached_parse(p, &mut c, cwd)?;
                Some((p.clone(), model))
            })
            .collect();
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
