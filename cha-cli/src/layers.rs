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
    let (name_to_paths, all_files) = index_files(files, cwd);
    let edges = resolve_edges(files, cwd, &name_to_paths);
    (edges, all_files)
}

fn index_files(
    files: &[PathBuf],
    cwd: &std::path::Path,
) -> (HashMap<String, Vec<String>>, Vec<String>) {
    let mut name_to_paths: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_files = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        all_files.push(rel.clone());
        if let Some(name) = path.file_name() {
            name_to_paths
                .entry(name.to_string_lossy().to_string())
                .or_default()
                .push(rel);
        }
    }
    (name_to_paths, all_files)
}

fn resolve_edges(
    files: &[PathBuf],
    cwd: &std::path::Path,
    name_to_paths: &HashMap<String, Vec<String>>,
) -> Vec<(String, String)> {
    let mut edges = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let file = SourceFile::new(path.clone(), content);
        let Some(model) = cha_parser::parse_file(&file) else {
            continue;
        };

        let src_dir = std::path::Path::new(&rel)
            .parent()
            .unwrap_or(std::path::Path::new(""));
        for imp in &model.imports {
            let target_name = imp.source.split('/').next_back().unwrap_or(&imp.source);
            if let Some(candidates) = name_to_paths.get(target_name)
                && let Some(target) = closest_candidate(candidates, src_dir)
                && *target != rel
            {
                edges.push((rel.clone(), target.clone()));
            }
        }
    }
    edges
}

fn closest_candidate<'a>(
    candidates: &'a [String],
    src_dir: &std::path::Path,
) -> Option<&'a String> {
    candidates.iter().min_by_key(|c| {
        let c_dir = std::path::Path::new(c.as_str())
            .parent()
            .unwrap_or(std::path::Path::new(""));
        usize::from(c_dir != src_dir)
    })
}

fn render_mermaid(layers: &[graph::LayerInfo], violations: &[graph::LayerViolation]) {
    println!("graph TD");
    let bands = [
        ("Stable", 0.0, 0.2),
        ("Core", 0.2, 0.4),
        ("Mid", 0.4, 0.6),
        ("Volatile", 0.6, 0.8),
        ("Leaf", 0.8, 1.01),
    ];
    for (label, lo, hi) in &bands {
        let members: Vec<&graph::LayerInfo> = layers
            .iter()
            .filter(|l| l.instability >= *lo && l.instability < *hi && l.fan_in + l.fan_out > 0)
            .collect();
        if members.is_empty() {
            continue;
        }
        let id = sanitize(label);
        println!("  subgraph {id}[\"{label}\"]");
        for l in &members {
            let nid = sanitize(&l.name);
            let short = l.name.split('/').next_back().unwrap_or(&l.name);
            println!(
                "    {nid}[\"{short} ({}f, I={:.2})\"]",
                l.file_count, l.instability
            );
        }
        println!("  end");
    }
    for v in violations {
        let from = sanitize(&v.from_module);
        let to = sanitize(&v.to_module);
        println!("  {from} -->|violation| {to}");
    }
}

fn render_dot(layers: &[graph::LayerInfo], violations: &[graph::LayerViolation]) {
    let active: std::collections::HashSet<&str> = violations
        .iter()
        .flat_map(|v| [v.from_module.as_str(), v.to_module.as_str()])
        .collect();
    let shown: Vec<&graph::LayerInfo> = layers
        .iter()
        .filter(|l| l.fan_in + l.fan_out > 0 || active.contains(l.name.as_str()))
        .collect();

    println!("digraph layers {{");
    println!("  rankdir=LR;");
    println!("  node [shape=box style=filled fillcolor=lightyellow fontsize=10];");
    println!("  edge [color=gray];");
    render_dot_bands(&shown);
    for v in violations {
        println!(
            "  {:?} -> {:?} [color=red penwidth=2];",
            v.from_module, v.to_module
        );
    }
    println!("}}");
}

fn render_dot_bands(shown: &[&graph::LayerInfo]) {
    const BANDS: &[(&str, f64, f64)] = &[
        ("Stable (I<0.2)", 0.0, 0.2),
        ("Core (0.2<=I<0.4)", 0.2, 0.4),
        ("Mid (0.4<=I<0.6)", 0.4, 0.6),
        ("Volatile (0.6<=I<0.8)", 0.6, 0.8),
        ("Leaf (I>=0.8)", 0.8, 1.01),
    ];
    for (i, &(label, lo, hi)) in BANDS.iter().enumerate() {
        let members: Vec<&&graph::LayerInfo> = shown
            .iter()
            .filter(|l| l.instability >= lo && l.instability < hi)
            .collect();
        if members.is_empty() {
            continue;
        }
        println!("  subgraph cluster_{i} {{");
        println!("    label={:?};", label);
        println!("    style=dashed;");
        for l in &members {
            let short = l.name.split('/').next_back().unwrap_or(&l.name);
            println!(
                "    {:?} [label={:?}];",
                l.name,
                format!("{}\n{}f, I={:.2}", short, l.file_count, l.instability)
            );
        }
        println!("  }}");
    }
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
