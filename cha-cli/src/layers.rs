use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use cha_core::SourceFile;

use crate::{analyze::filter_excluded, collect_files};

pub fn cmd_layers(paths: &[String], save: bool) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = cha_core::Config::load(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);

    let edges = build_dir_imports(&files, &cwd);
    let mut layers = infer_layers(&edges);
    layers.sort_by(|a, b| a.instability.partial_cmp(&b.instability).unwrap());

    println!("Inferred layers:\n");
    println!(
        "  {:<25} {:>7} {:>8} {:>12} {:>6}",
        "Directory", "fan-in", "fan-out", "Instability", "Layer"
    );
    println!("  {}", "-".repeat(62));
    for (i, l) in layers.iter().enumerate() {
        println!(
            "  {:<25} {:>7} {:>8} {:>12.2} {:>6}",
            l.name,
            l.fan_in,
            l.fan_out,
            l.instability,
            format!("L{i}")
        );
    }

    // Detect violations: lower instability dir importing higher instability dir
    let level: BTreeMap<&str, usize> = layers
        .iter()
        .enumerate()
        .map(|(i, l)| (l.name.as_str(), i))
        .collect();
    let mut violations = Vec::new();
    for (from_dir, to_dir) in &edges {
        if let (Some(&from_level), Some(&to_level)) =
            (level.get(from_dir.as_str()), level.get(to_dir.as_str()))
            && from_level < to_level
        {
            violations.push((from_dir.clone(), to_dir.clone(), from_level, to_level));
        }
    }

    if !violations.is_empty() {
        println!("\n⚠ {} potential layer violation(s):", violations.len());
        for (from, to, fl, tl) in &violations {
            println!("  {from} (L{fl}) imports {to} (L{tl})");
        }
    } else {
        println!("\n✅ No layer violations detected.");
    }

    if save {
        let layers_str = layers
            .iter()
            .enumerate()
            .map(|(i, l)| format!("{}:{}", l.name, i))
            .collect::<Vec<_>>()
            .join(",");
        println!("\nTo use in .cha.toml:\n");
        println!("[plugins.layer_violation]");
        println!("layers = \"{layers_str}\"");
    }
}

struct LayerInfo {
    name: String,
    fan_in: usize,
    fan_out: usize,
    instability: f64,
}

fn build_dir_imports(files: &[PathBuf], cwd: &std::path::Path) -> Vec<(String, String)> {
    use std::collections::HashMap;

    // First pass: build file→directory mapping for all project files
    let mut file_to_dir: HashMap<String, String> = HashMap::new();
    for path in files {
        let rel = path.strip_prefix(cwd).unwrap_or(path);
        let components: Vec<_> = rel.components().collect();
        // Use parent directory name as the "module"
        if components.len() >= 2 {
            let dir = components[components.len() - 2]
                .as_os_str()
                .to_string_lossy()
                .to_string();
            let filename = components
                .last()
                .unwrap()
                .as_os_str()
                .to_string_lossy()
                .to_string();
            file_to_dir.insert(filename, dir);
        }
    }

    // Second pass: for each file's imports, resolve target directory
    let mut edges = Vec::new();
    for path in files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let file = SourceFile::new(path.clone(), content);
        let model = match cha_parser::parse_file(&file) {
            Some(m) => m,
            None => continue,
        };
        let rel = path.strip_prefix(cwd).unwrap_or(path);
        let components: Vec<_> = rel.components().collect();
        if components.len() < 2 {
            continue;
        }
        let src_dir = components[components.len() - 2]
            .as_os_str()
            .to_string_lossy()
            .to_string();

        for imp in &model.imports {
            // Extract filename from import path (last component)
            let imp_file = imp.source.split('/').next_back().unwrap_or(&imp.source);
            if let Some(dst_dir) = file_to_dir.get(imp_file)
                && *dst_dir != src_dir
            {
                edges.push((src_dir.clone(), dst_dir.clone()));
            }
        }
    }
    edges
}

fn infer_layers(edges: &[(String, String)]) -> Vec<LayerInfo> {
    let mut fan_in: BTreeMap<&str, HashSet<&str>> = BTreeMap::new();
    let mut fan_out: BTreeMap<&str, HashSet<&str>> = BTreeMap::new();

    for (from, to) in edges {
        fan_out.entry(from).or_default().insert(to);
        fan_in.entry(to).or_default().insert(from);
    }

    let all_dirs: HashSet<&str> = fan_in.keys().chain(fan_out.keys()).copied().collect();
    all_dirs
        .into_iter()
        .map(|name| {
            let fi = fan_in.get(name).map(|s| s.len()).unwrap_or(0);
            let fo = fan_out.get(name).map(|s| s.len()).unwrap_or(0);
            let total = fi + fo;
            let instability = if total > 0 {
                fo as f64 / total as f64
            } else {
                0.5
            };
            LayerInfo {
                name: name.to_string(),
                fan_in: fi,
                fan_out: fo,
                instability,
            }
        })
        .collect()
}
