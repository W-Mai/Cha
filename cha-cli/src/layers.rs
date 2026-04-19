use std::collections::HashMap;
use std::path::PathBuf;

use cha_core::SourceFile;
use cha_core::graph;

use crate::{DepsFormat, analyze::filter_excluded, collect_files};

pub fn cmd_layers(paths: &[String], save: bool, format: &DepsFormat) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = cha_core::Config::load(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);

    let (file_imports, all_files) = build_import_edges(&files, &cwd);

    let modules = graph::infer_modules(&file_imports, &all_files);
    let (layers, violations) = graph::infer_layers(&modules, &file_imports);

    match format {
        DepsFormat::Dot => render_dot(&layers, &violations),
        DepsFormat::Mermaid => render_mermaid(&layers, &violations),
        DepsFormat::Json => render_json(&layers, &violations),
        DepsFormat::Plantuml => render_plantuml(&layers, &violations),
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

fn render_mermaid(layers: &[graph::LayerInfo], violations: &[graph::LayerViolation]) {
    println!("graph TD");
    for l in layers {
        let id = sanitize(&l.name);
        println!(
            "  {id}[\"{} (L{}, {}f, I={:.2})\"]",
            l.name, l.level, l.file_count, l.instability
        );
    }
    for v in violations {
        let from = sanitize(&v.from_module);
        let to = sanitize(&v.to_module);
        println!("  {from} -->|violation| {to}");
    }
    // Style violations in red
    for (i, _) in violations.iter().enumerate() {
        println!("  linkStyle {} stroke:red,stroke-width:2", layers.len() + i);
    }
}

fn render_dot(layers: &[graph::LayerInfo], violations: &[graph::LayerViolation]) {
    println!("digraph layers {{");
    println!("  rankdir=BT;");
    for l in layers {
        println!(
            "  \"{}\" [label=\"{} (L{}, {}f, I={:.2})\"];",
            l.name, l.name, l.level, l.file_count, l.instability
        );
    }
    for v in violations {
        println!(
            "  \"{}\" -> \"{}\" [color=red, label=\"violation\"];",
            v.from_module, v.to_module
        );
    }
    println!("}}");
}

fn render_json(layers: &[graph::LayerInfo], violations: &[graph::LayerViolation]) {
    let layers_json: Vec<serde_json::Value> = layers
        .iter()
        .map(|l| {
            serde_json::json!({
                "name": l.name, "level": l.level, "files": l.file_count,
                "fan_in": l.fan_in, "fan_out": l.fan_out, "instability": l.instability
            })
        })
        .collect();
    let violations_json: Vec<serde_json::Value> = violations
        .iter()
        .map(|v| {
            serde_json::json!({
                "from": v.from_module, "to": v.to_module,
                "from_level": v.from_level, "to_level": v.to_level
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "layers": layers_json, "violations": violations_json
        }))
        .unwrap_or_default()
    );
}

fn render_plantuml(layers: &[graph::LayerInfo], violations: &[graph::LayerViolation]) {
    println!("@startuml");
    for l in layers {
        println!("package \"{}\" as L{} {{", l.name, l.level);
        println!(
            "  note \"{}f, I={:.2}\" as N{}",
            l.file_count, l.instability, l.level
        );
        println!("}}");
    }
    for v in violations {
        println!("L{} --> L{} #red : violation", v.from_level, v.to_level);
    }
    println!("@enduml");
}

fn sanitize(s: &str) -> String {
    s.replace(['/', '.', '-'], "_")
}
