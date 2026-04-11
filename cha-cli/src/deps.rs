use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::SourceFile;

use crate::{DepsDepth, DepsFormat, analyze::filter_excluded, collect_files};

pub fn cmd_deps(paths: &[String], format: &DepsFormat, depth: &DepsDepth) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = cha_core::Config::load(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);

    let mut edges = build_dep_edges(&files, &cwd);

    if matches!(depth, DepsDepth::Dir) {
        edges = aggregate_to_dirs(edges);
    }

    let cycles = detect_cycles(&edges);

    match format {
        DepsFormat::Dot => print_dot(&edges, &cycles),
        DepsFormat::Json => print_deps_json(&edges, &cycles),
        DepsFormat::Mermaid => print_mermaid(&edges, &cycles),
    }

    if !cycles.is_empty() {
        eprintln!("\n⚠ {} circular dependency(ies) detected", cycles.len());
    }
}

const SKIP_PREFIXES: &[&str] = &["std::", "core::", "alloc::", "crate::", "super::", "self::"];

fn build_dep_edges(files: &[PathBuf], cwd: &Path) -> Vec<(String, String)> {
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
            edges.push((src.clone(), imp.source.clone()));
        }
    }
    edges
}

fn aggregate_to_dirs(edges: Vec<(String, String)>) -> Vec<(String, String)> {
    let dir_of = |p: &str| {
        Path::new(p)
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string()
    };
    let known: HashSet<String> = edges.iter().map(|(a, _)| a.clone()).collect();
    let mut result: Vec<(String, String)> = edges
        .into_iter()
        .filter(|(_, b)| known.contains(b))
        .map(|(a, b)| (dir_of(&a), dir_of(&b)))
        .filter(|(a, b)| a != b)
        .collect();
    result.sort();
    result.dedup();
    result
}

fn detect_cycles(edges: &[(String, String)]) -> Vec<(String, String)> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (a, b) in edges {
        adj.entry(a.as_str()).or_default().push(b.as_str());
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

fn print_dot(edges: &[(String, String)], cycles: &[(String, String)]) {
    let cycle_set: HashSet<(&str, &str)> = cycles
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    println!("digraph deps {{");
    println!("  rankdir=LR;");
    for (a, b) in edges {
        let attr = if cycle_set.contains(&(a.as_str(), b.as_str())) {
            " [color=red, penwidth=2]"
        } else {
            ""
        };
        println!("  \"{}\" -> \"{}\"{};", a, b, attr);
    }
    println!("}}");
}

fn print_deps_json(edges: &[(String, String)], cycles: &[(String, String)]) {
    let nodes: BTreeSet<&str> = edges
        .iter()
        .flat_map(|(a, b)| [a.as_str(), b.as_str()])
        .collect();
    let json = serde_json::json!({
        "nodes": nodes,
        "edges": edges.iter().map(|(a, b)| serde_json::json!({"from": a, "to": b})).collect::<Vec<_>>(),
        "cycles": cycles.iter().map(|(a, b)| serde_json::json!({"from": a, "to": b})).collect::<Vec<_>>(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );
}

fn print_mermaid(edges: &[(String, String)], cycles: &[(String, String)]) {
    let cycle_set: HashSet<(&str, &str)> = cycles
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let sanitize = |s: &str| s.replace(|c: char| !c.is_alphanumeric(), "_");
    println!("graph LR");
    for (i, (a, b)) in edges.iter().enumerate() {
        println!(
            "  {}[\"{}\"] --> {}[\"{}\"]",
            sanitize(a),
            a,
            sanitize(b),
            b
        );
        if cycle_set.contains(&(a.as_str(), b.as_str())) {
            println!("  linkStyle {} stroke:red,stroke-width:3", i);
        }
    }
}
