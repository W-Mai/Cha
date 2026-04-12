use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::SourceFile;

use crate::{DepsDepth, DepsFormat, DepsType, analyze::filter_excluded, collect_files};

pub fn cmd_deps(
    paths: &[String],
    format: &DepsFormat,
    depth: &DepsDepth,
    graph_type: &DepsType,
    filter: Option<&str>,
) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = cha_core::Config::load(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);

    let edges = match graph_type {
        DepsType::Imports => build_import_graph(&files, &cwd, depth),
        DepsType::Classes => build_class_graph(&files),
        DepsType::Calls => {
            eprintln!("--type calls is not yet implemented");
            return;
        }
    };

    let edges = apply_filter(edges, filter);
    let cycles = detect_cycles(&edges);
    render(&edges, &cycles, format);

    if !cycles.is_empty() {
        eprintln!("\n⚠ {} circular dependency(ies) detected", cycles.len());
    }
}

// ── Edge with optional label ──

struct Edge {
    from: String,
    to: String,
    label: Option<String>,
}

fn apply_filter(edges: Vec<Edge>, filter: Option<&str>) -> Vec<Edge> {
    let Some(name) = filter else {
        return edges;
    };
    edges
        .into_iter()
        .filter(|e| e.from.contains(name) || e.to.contains(name))
        .collect()
}

// ── Import graph (existing) ──

const SKIP_PREFIXES: &[&str] = &["std::", "core::", "alloc::", "crate::", "super::", "self::"];

fn build_import_graph(files: &[PathBuf], cwd: &Path, depth: &DepsDepth) -> Vec<Edge> {
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
        let src = path
            .strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        for imp in &model.imports {
            if SKIP_PREFIXES.iter().any(|p| imp.source.starts_with(p)) {
                continue;
            }
            edges.push(Edge {
                from: src.clone(),
                to: imp.source.clone(),
                label: None,
            });
        }
    }
    if matches!(depth, DepsDepth::Dir) {
        aggregate_to_dirs(edges)
    } else {
        edges
    }
}

fn aggregate_to_dirs(edges: Vec<Edge>) -> Vec<Edge> {
    let dir_of = |p: &str| {
        Path::new(p)
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string()
    };
    let known: HashSet<String> = edges.iter().map(|e| e.from.clone()).collect();
    let mut seen = HashSet::new();
    edges
        .into_iter()
        .filter(|e| known.contains(&e.to))
        .filter_map(|e| {
            let a = dir_of(&e.from);
            let b = dir_of(&e.to);
            if a == b {
                return None;
            }
            let key = format!("{a}->{b}");
            if seen.insert(key) {
                Some(Edge {
                    from: a,
                    to: b,
                    label: None,
                })
            } else {
                None
            }
        })
        .collect()
}

// ── Class graph ──

fn parse_all_models(files: &[PathBuf]) -> Vec<cha_core::SourceModel> {
    files
        .iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            let file = SourceFile::new(path.clone(), content);
            cha_parser::parse_file(&file)
        })
        .collect()
}

fn build_class_graph(files: &[PathBuf]) -> Vec<Edge> {
    let models = parse_all_models(files);
    let interfaces: HashSet<String> = models
        .iter()
        .flat_map(|m| &m.classes)
        .filter(|c| c.is_interface)
        .map(|c| c.name.clone())
        .collect();
    models
        .iter()
        .flat_map(|m| &m.classes)
        .filter_map(|class| {
            let parent = class.parent_name.as_ref()?;
            let label = if interfaces.contains(parent) {
                "implements"
            } else {
                "extends"
            };
            Some(Edge {
                from: class.name.clone(),
                to: parent.clone(),
                label: Some(label.to_string()),
            })
        })
        .collect()
}

// ── Cycle detection ──

fn detect_cycles(edges: &[Edge]) -> Vec<(String, String)> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in edges {
        adj.entry(e.from.as_str()).or_default().push(e.to.as_str());
    }
    let mut cycle_edges = Vec::new();
    let mut visited = HashSet::new();
    let mut on_stack = HashSet::new();
    for node in adj.keys() {
        dfs_cycle(node, &adj, &mut visited, &mut on_stack, &mut cycle_edges);
    }
    cycle_edges
}

fn dfs_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    on_stack: &mut HashSet<&'a str>,
    cycles: &mut Vec<(String, String)>,
) {
    if on_stack.contains(node) || visited.contains(node) {
        return;
    }
    visited.insert(node);
    on_stack.insert(node);
    if let Some(neighbors) = adj.get(node) {
        for &next in neighbors {
            if on_stack.contains(next) {
                cycles.push((node.to_string(), next.to_string()));
            } else if !visited.contains(next) {
                dfs_cycle(next, adj, visited, on_stack, cycles);
            }
        }
    }
    on_stack.remove(node);
}

// ── Rendering ──

fn render(edges: &[Edge], cycles: &[(String, String)], format: &DepsFormat) {
    match format {
        DepsFormat::Dot => print_dot(edges, cycles),
        DepsFormat::Json => print_json(edges, cycles),
        DepsFormat::Mermaid => print_mermaid(edges, cycles),
    }
}

fn print_dot(edges: &[Edge], cycles: &[(String, String)]) {
    let cycle_set: HashSet<(&str, &str)> = cycles
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    println!("digraph deps {{");
    println!("  rankdir=LR;");
    for e in edges {
        let mut attrs = Vec::new();
        if cycle_set.contains(&(e.from.as_str(), e.to.as_str())) {
            attrs.push("color=red".to_string());
            attrs.push("penwidth=2".to_string());
        }
        if let Some(label) = &e.label {
            attrs.push(format!("label=\"{label}\""));
        }
        let attr_str = if attrs.is_empty() {
            String::new()
        } else {
            format!(" [{}]", attrs.join(", "))
        };
        println!("  \"{}\" -> \"{}\"{};", e.from, e.to, attr_str);
    }
    println!("}}");
}

fn print_json(edges: &[Edge], cycles: &[(String, String)]) {
    let nodes: BTreeSet<&str> = edges
        .iter()
        .flat_map(|e| [e.from.as_str(), e.to.as_str()])
        .collect();
    let json = serde_json::json!({
        "nodes": nodes,
        "edges": edges.iter().map(|e| {
            let mut obj = serde_json::json!({"from": e.from, "to": e.to});
            if let Some(label) = &e.label {
                obj["label"] = serde_json::json!(label);
            }
            obj
        }).collect::<Vec<_>>(),
        "cycles": cycles.iter().map(|(a, b)| serde_json::json!({"from": a, "to": b})).collect::<Vec<_>>(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );
}

fn print_mermaid(edges: &[Edge], cycles: &[(String, String)]) {
    let cycle_set: HashSet<(&str, &str)> = cycles
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let sanitize = |s: &str| s.replace(|c: char| !c.is_alphanumeric(), "_");
    println!("graph LR");
    for (i, e) in edges.iter().enumerate() {
        let arrow = if e.label.is_some() {
            format!("-->|{}|", e.label.as_deref().unwrap_or(""))
        } else {
            "-->".to_string()
        };
        println!(
            "  {}[\"{}\"] {} {}[\"{}\"]",
            sanitize(&e.from),
            e.from,
            arrow,
            sanitize(&e.to),
            e.to
        );
        if cycle_set.contains(&(e.from.as_str(), e.to.as_str())) {
            println!("  linkStyle {} stroke:red,stroke-width:3", i);
        }
    }
}
