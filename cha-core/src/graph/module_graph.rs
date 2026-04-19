use std::collections::{HashMap, HashSet};

/// A group of tightly-coupled files inferred from the import graph.
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub files: Vec<String>,
}

/// Infer modules from file-level import edges.
///
/// Algorithm:
/// 1. Exclusive-dependency merge: if file B has fan-in=1 (only imported by A), merge B into A's module.
///    Repeat until no new merges.
/// 2. Directory fallback: remaining unmerged files are grouped by parent directory.
pub fn infer_modules(file_imports: &[(String, String)], all_files: &[String]) -> Vec<Module> {
    // Compute fan-in for each file
    let mut fan_in: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (from, to) in file_imports {
        fan_in.entry(to.as_str()).or_default().insert(from.as_str());
    }

    // Union-Find: each file starts as its own module
    let mut parent: HashMap<String, String> =
        all_files.iter().map(|f| (f.clone(), f.clone())).collect();

    // Step 1: exclusive-dependency merge
    loop {
        let mut changed = false;
        for (_, to) in file_imports {
            let importers = match fan_in.get(to.as_str()) {
                Some(s) if s.len() == 1 => s,
                _ => continue,
            };
            let sole_importer = importers.iter().next().unwrap();
            let root_a = find(&parent, sole_importer);
            let root_b = find(&parent, to);
            if root_a != root_b {
                parent.insert(root_b, root_a.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Step 2: directory fallback — merge modules that share the same parent directory
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for file in all_files {
        let root = find(&parent, file);
        groups.entry(root).or_default().push(file.clone());
    }

    // Group all clusters by their common parent directory, then merge
    let mut dir_groups: HashMap<String, Vec<String>> = HashMap::new();
    for (_, members) in groups {
        let dir = common_prefix(&members);
        dir_groups.entry(dir).or_default().extend(members);
    }

    dir_groups
        .into_values()
        .map(|files| {
            let name = common_prefix(&files);
            Module { name, files }
        })
        .collect()
}

fn find(parent: &HashMap<String, String>, x: &str) -> String {
    let mut cur = x.to_string();
    while let Some(p) = parent.get(&cur) {
        if p == &cur {
            break;
        }
        cur = p.clone();
    }
    cur
}

fn common_prefix(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    if paths.len() == 1 {
        return std::path::Path::new(&paths[0])
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| paths[0].clone());
    }
    let parts: Vec<Vec<&str>> = paths.iter().map(|p| p.split('/').collect()).collect();
    let mut prefix = Vec::new();
    for i in 0.. {
        let first = match parts[0].get(i) {
            Some(s) => s,
            None => break,
        };
        if parts.iter().all(|p| p.get(i) == Some(first)) {
            prefix.push(*first);
        } else {
            break;
        }
    }
    if prefix.is_empty() {
        "(root)".to_string()
    } else {
        prefix.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_dep_merges() {
        let files = vec!["src/a.rs".into(), "src/b.rs".into(), "src/c.rs".into()];
        // b only imported by a → merge
        // c imported by a and b → not merged
        let imports = vec![
            ("src/a.rs".into(), "src/b.rs".into()),
            ("src/a.rs".into(), "src/c.rs".into()),
            ("src/b.rs".into(), "src/c.rs".into()),
        ];
        let modules = infer_modules(&imports, &files);
        // a and b should be in same module, c separate
        let ab_mod = modules
            .iter()
            .find(|m| m.files.contains(&"src/a.rs".to_string()));
        assert!(ab_mod.is_some());
        assert!(ab_mod.unwrap().files.contains(&"src/b.rs".to_string()));
    }

    #[test]
    fn directory_fallback() {
        let files = vec![
            "src/core/x.rs".into(),
            "src/core/y.rs".into(),
            "src/util/z.rs".into(),
        ];
        // No exclusive deps, so directory fallback
        let imports = vec![];
        let modules = infer_modules(&imports, &files);
        // core/x and core/y should be in same module
        let core_mod = modules
            .iter()
            .find(|m| m.files.contains(&"src/core/x.rs".to_string()));
        assert!(core_mod.is_some());
        assert!(
            core_mod
                .unwrap()
                .files
                .contains(&"src/core/y.rs".to_string())
        );
    }
}
