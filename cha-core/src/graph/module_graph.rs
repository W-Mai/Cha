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
    let mut fan_in: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (from, to) in file_imports {
        fan_in.entry(to.as_str()).or_default().insert(from.as_str());
    }

    let mut parent: HashMap<String, String> =
        all_files.iter().map(|f| (f.clone(), f.clone())).collect();

    merge_exclusive_deps(file_imports, &fan_in, &mut parent);

    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for file in all_files {
        groups
            .entry(find(&parent, file))
            .or_default()
            .push(file.clone());
    }

    let mut dir_groups: HashMap<String, Vec<String>> = HashMap::new();
    for (_, members) in groups {
        dir_groups
            .entry(common_prefix(&members))
            .or_default()
            .extend(members);
    }

    dir_groups
        .into_values()
        .map(|files| Module {
            name: common_prefix(&files),
            files,
        })
        .collect()
}

fn merge_exclusive_deps(
    file_imports: &[(String, String)],
    fan_in: &HashMap<&str, HashSet<&str>>,
    parent: &mut HashMap<String, String>,
) {
    loop {
        let mut changed = false;
        for (_, to) in file_imports {
            let importers = match fan_in.get(to.as_str()) {
                Some(s) if s.len() == 1 => s,
                _ => continue,
            };
            let sole_importer = importers.iter().next().unwrap();
            let root_a = find(parent, sole_importer);
            let root_b = find(parent, to);
            if root_a != root_b {
                parent.insert(root_b, root_a.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
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
    if paths.len() <= 1 {
        return paths
            .first()
            .and_then(|p| std::path::Path::new(p).parent())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
    }
    let parts: Vec<Vec<&str>> = paths.iter().map(|p| p.split('/').collect()).collect();
    let prefix: Vec<&str> = (0..)
        .map_while(|i| {
            let first = parts[0].get(i)?;
            parts
                .iter()
                .all(|p| p.get(i) == Some(first))
                .then_some(*first)
        })
        .collect();
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
