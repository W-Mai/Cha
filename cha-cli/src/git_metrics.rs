//! Git-backed project-level metrics: Martin's instability (unstable
//! dependency chains), bus-factor (single-author files), and the test-
//! to-production line ratio. Kept together because none of these need
//! parsed AST models — they operate on either the file list + import
//! cache (instability) or `git log` output (bus factor, test ratio).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::Finding;

/// Flag files whose imports reach a module that is *less* stable than
/// themselves — violates Martin's Stable Dependencies Principle. Uses
/// the parse cache's `imports` meta so no parsing happens here.
pub fn detect_unstable_deps(
    files: &[PathBuf],
    cwd: &Path,
    cache: &std::sync::Mutex<cha_core::ProjectCache>,
) -> Vec<Finding> {
    let file_imports = build_file_imports(files, cwd, cache);
    let known: HashSet<&str> = file_imports.keys().map(|s| s.as_str()).collect();
    let ca = compute_afferent(&file_imports, &known);

    let instability = |file: &str| -> f64 {
        let ce = file_imports.get(file).map(|v| v.len()).unwrap_or(0) as f64;
        let ca_val = ca.get(file).copied().unwrap_or(0) as f64;
        if ce + ca_val == 0.0 {
            0.0
        } else {
            ce / (ca_val + ce)
        }
    };

    let mut name_to_path: HashMap<&str, &str> = HashMap::new();
    for &k in &known {
        let basename = k.rsplit('/').next().unwrap_or(k);
        name_to_path.entry(basename).or_insert(k);
    }
    let resolve = |imp: &str| -> Option<&str> {
        let basename = imp.rsplit('/').next().unwrap_or(imp);
        name_to_path
            .get(basename)
            .copied()
            .or_else(|| known.get(imp).copied())
    };

    file_imports
        .iter()
        .filter_map(|(file, imports)| {
            let my_i = instability(file);
            let (target, ti) = imports.iter().find_map(|imp| {
                let t = resolve(imp)?;
                let ti = instability(t);
                (my_i < ti && (ti - my_i) > 0.2).then_some((t, ti))
            })?;
            Some(make_unstable_finding(file, my_i, target, ti))
        })
        .collect()
}

fn make_unstable_finding(file: &str, my_i: f64, target: &str, ti: f64) -> Finding {
    Finding {
        smell_name: "unstable_dependency".into(),
        category: cha_core::SmellCategory::Couplers,
        severity: cha_core::Severity::Hint,
        location: cha_core::Location {
            path: PathBuf::from(file),
            start_line: 1,
            end_line: 1,
            name: None,
            ..Default::default()
        },
        message: format!(
            "`{file}` (I={my_i:.2}) depends on `{target}` (I={ti:.2}) which is less stable"
        ),
        suggested_refactorings: vec![
            "Depend on abstractions".into(),
            "Stable Dependencies Principle".into(),
        ],
        ..Default::default()
    }
}

fn build_file_imports(
    files: &[PathBuf],
    cwd: &Path,
    cache: &std::sync::Mutex<cha_core::ProjectCache>,
) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    let mut c = match cache.lock() {
        Ok(c) => c,
        Err(_) => return map,
    };
    for path in files {
        let rel = path
            .strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        // Fast path: read imports directly from meta (no parse needed)
        if let Some(imports) = c.get_imports(&rel)
            && !imports.is_empty()
        {
            map.insert(rel, imports.to_vec());
            continue;
        }
        if let Some((rel, model)) = crate::cached_parse(path, &mut c, cwd) {
            map.insert(
                rel,
                model.imports.iter().map(|i| i.source.clone()).collect(),
            );
        }
    }
    map
}

fn compute_afferent<'a>(
    file_imports: &'a HashMap<String, Vec<String>>,
    known: &HashSet<&'a str>,
) -> HashMap<&'a str, usize> {
    // Reverse index: filename → full path, so each import can be O(1)
    // resolved whether it's written as a basename or a full relative path.
    let mut name_to_path: HashMap<&str, &str> = HashMap::new();
    for &k in known {
        let basename = k.rsplit('/').next().unwrap_or(k);
        name_to_path.entry(basename).or_insert(k);
    }
    let mut ca = HashMap::new();
    for imports in file_imports.values() {
        for imp in imports {
            let basename = imp.rsplit('/').next().unwrap_or(imp.as_str());
            if let Some(&k) = name_to_path.get(basename) {
                *ca.entry(k).or_default() += 1;
            } else if let Some(&k) = known.get(imp.as_str()) {
                *ca.entry(k).or_default() += 1;
            }
        }
    }
    ca
}

/// Flag files with bus factor = 1 (single git author). Skips tiny files
/// where a single-author history is unremarkable.
///
/// ## References
///
/// [1] N. Nagappan et al., "The influence of organizational structure on
///     software quality," ICSE 2008. doi: 10.1145/1368088.1368122.
pub fn detect_bus_factor(files: &[PathBuf], cwd: &Path) -> Vec<Finding> {
    let file_authors = git_file_authors();

    files
        .iter()
        .filter_map(|path| {
            let rel = path.strip_prefix(cwd).unwrap_or(path);
            let authors = file_authors.get(rel.to_str()?)?;
            (authors.len() == 1 && path.metadata().map(|m| m.len() > 500).unwrap_or(false)).then(
                || Finding {
                    smell_name: "bus_factor".into(),
                    category: cha_core::SmellCategory::ChangePreventers,
                    severity: cha_core::Severity::Hint,
                    location: cha_core::Location {
                        path: rel.to_path_buf(),
                        start_line: 1,
                        end_line: 1,
                        name: None,
                        ..Default::default()
                    },
                    message: format!(
                        "`{}` has only 1 contributor — bus factor risk",
                        rel.display()
                    ),
                    suggested_refactorings: vec!["Pair programming".into(), "Code review".into()],
                    ..Default::default()
                },
            )
        })
        .collect()
}

/// Single `git log` call to build file → authors map.
fn git_file_authors() -> HashMap<String, HashSet<String>> {
    let output = std::process::Command::new("git")
        .args(["log", "--format=%aN", "-n", "200", "--name-only"])
        .output()
        .ok();
    let Some(output) = output else {
        return Default::default();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut file_authors: HashMap<String, HashSet<String>> = HashMap::new();
    let mut current_author = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.contains('/') && !line.contains('.') {
            current_author = line.to_string();
        } else if !current_author.is_empty() {
            file_authors
                .entry(line.to_string())
                .or_default()
                .insert(current_author.clone());
        }
    }
    file_authors
}

/// Flag projects whose test-to-production line ratio is below 50 %.
pub fn check_test_ratio(files: &[PathBuf]) -> Vec<Finding> {
    let (mut test_lines, mut prod_lines) = (0usize, 0usize);
    for f in files {
        let lines = std::fs::read_to_string(f)
            .map(|c| c.lines().count())
            .unwrap_or(0);
        if f.to_string_lossy().contains("test") || f.to_string_lossy().contains("spec") {
            test_lines += lines;
        } else {
            prod_lines += lines;
        }
    }
    if prod_lines == 0 || (test_lines as f64 / prod_lines as f64) >= 0.5 {
        return vec![];
    }
    let ratio = test_lines as f64 / prod_lines as f64;
    vec![Finding {
        smell_name: "low_test_ratio".into(),
        category: cha_core::SmellCategory::Dispensables,
        severity: cha_core::Severity::Hint,
        location: cha_core::Location {
            path: PathBuf::from("."),
            start_line: 1,
            end_line: 1,
            name: None,
            ..Default::default()
        },
        message: format!(
            "Test-to-code ratio is {:.0}% ({test_lines} test / {prod_lines} production lines)",
            ratio * 100.0
        ),
        suggested_refactorings: vec!["Add unit tests".into()],
        ..Default::default()
    }]
}
