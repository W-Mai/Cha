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
    exact: bool,
) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = cha_core::Config::load(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);

    let edges = match graph_type {
        DepsType::Imports => build_import_graph(&files, &cwd, depth),
        DepsType::Classes => build_class_graph(&files),
        DepsType::Calls => build_call_graph(&files),
    };

    let edges = apply_filter(edges, filter, exact);
    let cycles = detect_cycles(&edges);
    let style = match graph_type {
        DepsType::Imports => CycleStyle::CircularDep,
        DepsType::Calls => CycleStyle::Recursion,
        DepsType::Classes => CycleStyle::CircularDep,
    };
    render(&edges, &cycles, format, &style);

    if !cycles.is_empty() {
        let label = match style {
            CycleStyle::CircularDep => "circular dependency(ies)",
            CycleStyle::Recursion => "recursive call(s)",
        };
        eprintln!("\n⚠ {} {label} detected", cycles.len());
    }
}

// ── Edge with optional label ──

enum CycleStyle {
    CircularDep,
    Recursion,
}

struct Edge {
    from: String,
    to: String,
    label: Option<String>,
}

fn apply_filter(edges: Vec<Edge>, filter: Option<&str>, exact: bool) -> Vec<Edge> {
    let Some(pattern) = filter else {
        return edges;
    };
    let re = regex::Regex::new(pattern).unwrap_or_else(|_| {
        // Fallback: treat as literal if invalid regex
        regex::Regex::new(&regex::escape(pattern)).unwrap()
    });
    let matches = |s: &str| re.is_match(s);
    if exact {
        return edges
            .into_iter()
            .filter(|e| matches(&e.from) || matches(&e.to))
            .collect();
    }
    let matched = expand_connected(&edges, &re);
    edges
        .into_iter()
        .filter(|e| matched.contains(&e.from) && matched.contains(&e.to))
        .collect()
}

fn expand_connected(edges: &[Edge], re: &regex::Regex) -> HashSet<String> {
    let mut matched: HashSet<String> = edges
        .iter()
        .filter(|e| re.is_match(&e.from) || re.is_match(&e.to))
        .flat_map(|e| [e.from.clone(), e.to.clone()])
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for e in edges {
            let has_from = matched.contains(&e.from);
            let has_to = matched.contains(&e.to);
            if has_from && !has_to {
                matched.insert(e.to.clone());
                changed = true;
            } else if has_to && !has_from {
                matched.insert(e.from.clone());
                changed = true;
            }
        }
    }
    matched
}

// ── Import graph (existing) ──

const SKIP_PREFIXES: &[&str] = &["std::", "core::", "alloc::", "crate::", "super::", "self::"];

fn build_import_graph(files: &[PathBuf], cwd: &Path, depth: &DepsDepth) -> Vec<Edge> {
    let pb = crate::new_progress_bar(files.len() as u64);
    let mut edges = Vec::new();
    for path in files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                pb.inc(1);
                continue;
            }
        };
        let file = SourceFile::new(path.clone(), content);
        let model = match cha_parser::parse_file(&file) {
            Some(m) => m,
            None => {
                pb.inc(1);
                continue;
            }
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
        pb.inc(1);
    }
    pb.finish_and_clear();
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
    let pb = crate::new_progress_bar(files.len() as u64);
    let result = files
        .iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            let file = SourceFile::new(path.clone(), content);
            let model = cha_parser::parse_file(&file);
            pb.inc(1);
            model
        })
        .collect();
    pb.finish_and_clear();
    result
}

struct ClassContext {
    all_names: HashSet<String>,
    interfaces: HashSet<String>,
    aliases: HashMap<String, String>,
    reverse: HashMap<String, String>,
}

impl ClassContext {
    fn from_files(files: &[PathBuf], models: &[cha_core::SourceModel]) -> Self {
        let aliases = collect_typedef_aliases(files);
        let reverse = aliases
            .iter()
            .map(|(a, o)| (o.clone(), a.clone()))
            .collect();
        let mut all_names: HashSet<String> = models
            .iter()
            .flat_map(|m| &m.classes)
            .map(|c| c.name.clone())
            .collect();
        all_names.extend(aliases.keys().cloned());
        let interfaces = models
            .iter()
            .flat_map(|m| &m.classes)
            .filter(|c| c.is_interface)
            .map(|c| c.name.clone())
            .collect();
        Self {
            all_names,
            interfaces,
            aliases,
            reverse,
        }
    }

    fn display_name(&self, name: &str) -> String {
        self.reverse
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }
}

fn build_class_graph(files: &[PathBuf]) -> Vec<Edge> {
    let models = parse_all_models(files);
    let ctx = ClassContext::from_files(files, &models);
    models
        .iter()
        .flat_map(|m| &m.classes)
        .filter_map(|class| {
            let parent = class.parent_name.as_ref()?;
            let resolved = ctx.aliases.get(parent.as_str()).unwrap_or(parent);
            if !ctx.all_names.contains(resolved) && !ctx.all_names.contains(parent) {
                return None;
            }
            let label = if ctx.interfaces.contains(resolved) || ctx.interfaces.contains(parent) {
                "implements"
            } else {
                "extends"
            };
            Some(Edge {
                from: ctx.display_name(&class.name),
                to: parent.clone(),
                label: Some(label.to_string()),
            })
        })
        .collect()
}

/// Scan files for `typedef struct X Y;` patterns to build alias map (Y -> X).
fn collect_typedef_aliases(files: &[PathBuf]) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for path in files {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        for line in content.lines() {
            let trimmed = line.trim();
            // Match: typedef struct _X X;
            if let Some(rest) = trimmed.strip_prefix("typedef struct ") {
                let parts: Vec<&str> = rest.trim_end_matches(';').split_whitespace().collect();
                if parts.len() == 2 {
                    aliases.insert(parts[1].to_string(), parts[0].to_string());
                }
            }
        }
    }
    aliases
}

// ── Call graph ──

fn build_call_graph(files: &[PathBuf]) -> Vec<Edge> {
    let models = parse_all_models(files);
    let known: HashSet<String> = models
        .iter()
        .flat_map(|m| &m.functions)
        .map(|f| f.name.clone())
        .collect();
    models
        .iter()
        .flat_map(|m| &m.functions)
        .flat_map(|f| {
            f.called_functions
                .iter()
                .filter_map(|callee| {
                    // Extract the last segment (e.g. "obj.method" -> "method", "func" -> "func")
                    let name = callee.rsplit('.').next().unwrap_or(callee);
                    known.contains(name).then(|| Edge {
                        from: f.name.clone(),
                        to: name.to_string(),
                        label: None,
                    })
                })
                .collect::<Vec<_>>()
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

fn render(edges: &[Edge], cycles: &[(String, String)], format: &DepsFormat, style: &CycleStyle) {
    match format {
        DepsFormat::Dot => print_dot(edges, cycles, style),
        DepsFormat::Json => print_json(edges, cycles),
        DepsFormat::Mermaid => print_mermaid(edges, cycles, style),
    }
}

fn print_dot(edges: &[Edge], cycles: &[(String, String)], style: &CycleStyle) {
    let cycle_set: HashSet<(&str, &str)> = cycles
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    println!("digraph deps {{");
    println!("  rankdir=LR;");
    for e in edges {
        let mut attrs = Vec::new();
        if cycle_set.contains(&(e.from.as_str(), e.to.as_str())) {
            match style {
                CycleStyle::CircularDep => {
                    attrs.push("color=red".into());
                    attrs.push("penwidth=2".into());
                }
                CycleStyle::Recursion => {
                    attrs.push("color=blue".into());
                    attrs.push("style=dashed".into());
                }
            }
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

fn mermaid_arrow(label: &Option<String>, is_recursion: bool) -> String {
    match (label, is_recursion) {
        (Some(l), _) => format!("-->|{l}|"),
        (None, true) => "-.->".into(),
        (None, false) => "-->".into(),
    }
}

fn print_mermaid(edges: &[Edge], cycles: &[(String, String)], style: &CycleStyle) {
    let cycle_set: HashSet<(&str, &str)> = cycles
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let sanitize = |s: &str| s.replace(|c: char| !c.is_alphanumeric(), "_");
    let color = match style {
        CycleStyle::CircularDep => "red",
        CycleStyle::Recursion => "blue",
    };
    println!("graph LR");
    for (i, e) in edges.iter().enumerate() {
        let is_cycle = cycle_set.contains(&(e.from.as_str(), e.to.as_str()));
        let arrow = mermaid_arrow(&e.label, is_cycle && matches!(style, CycleStyle::Recursion));
        println!(
            "  {}[\"{}\"] {} {}[\"{}\"]",
            sanitize(&e.from),
            e.from,
            arrow,
            sanitize(&e.to),
            e.to
        );
        if is_cycle {
            println!("  linkStyle {} stroke:{color},stroke-width:2", i);
        }
    }
}
