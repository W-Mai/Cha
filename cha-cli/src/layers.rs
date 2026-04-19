use std::collections::HashMap;
use std::path::PathBuf;

use cha_core::SourceFile;
use cha_core::graph;

use crate::{analyze::filter_excluded, collect_files};

pub fn cmd_layers(paths: &[String], save: bool) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = cha_core::Config::load(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);

    let (file_imports, all_files) = build_import_edges(&files, &cwd);

    let modules = graph::infer_modules(&file_imports, &all_files);
    let (layers, violations) = graph::infer_layers(&modules, &file_imports);

    println!(
        "Inferred {} modules, {} layers:\n",
        modules.len(),
        layers.len()
    );
    println!(
        "  {:<40} {:>5} {:>7} {:>8} {:>6}",
        "Module", "files", "fan-in", "fan-out", "I"
    );
    println!("  {}", "-".repeat(70));
    for l in &layers {
        println!(
            "  {:<40} {:>5} {:>7} {:>8} {:>6.2}  L{}",
            l.name, l.file_count, l.fan_in, l.fan_out, l.instability, l.level
        );
    }

    if violations.is_empty() {
        println!("\n✅ No layer violations detected.");
    } else {
        println!("\n⚠ {} potential layer violation(s):", violations.len());
        for v in &violations {
            println!(
                "  {} (L{}) → {} (L{})",
                v.from_module, v.from_level, v.to_module, v.to_level
            );
        }
    }

    if save {
        let layers_str = layers
            .iter()
            .map(|l| format!("{}:{}", l.name, l.level))
            .collect::<Vec<_>>()
            .join(",");
        println!("\nTo use in .cha.toml:\n");
        println!("[plugins.layer_violation]");
        println!("layers = \"{layers_str}\"");
    }
}

fn build_import_edges(
    files: &[PathBuf],
    cwd: &std::path::Path,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut name_to_paths: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_files = Vec::new();

    for path in files {
        let rel_str = path
            .strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        all_files.push(rel_str.clone());
        if let Some(name) = path.file_name() {
            name_to_paths
                .entry(name.to_string_lossy().to_string())
                .or_default()
                .push(rel_str);
        }
    }

    let mut edges = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let file = SourceFile::new(path.clone(), content);
        let model = match cha_parser::parse_file(&file) {
            Some(m) => m,
            None => continue,
        };

        let src_dir = std::path::Path::new(&rel)
            .parent()
            .unwrap_or(std::path::Path::new(""));

        for imp in &model.imports {
            let target_name = imp.source.split('/').next_back().unwrap_or(&imp.source);
            if let Some(candidates) = name_to_paths.get(target_name)
                && let Some(target) = candidates.iter().min_by_key(|c| {
                    let c_dir = std::path::Path::new(c.as_str())
                        .parent()
                        .unwrap_or(std::path::Path::new(""));
                    if c_dir == src_dir { 0usize } else { 1 }
                })
                && *target != rel
            {
                edges.push((rel.clone(), target.clone()));
            }
        }
    }
    (edges, all_files)
}
