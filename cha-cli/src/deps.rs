use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::{
    DepsDepth, DepsDirection, DepsFormat, DepsType, analyze::filter_excluded, collect_files,
};

// cha:ignore high_complexity
#[allow(clippy::too_many_arguments)]
// cha:ignore long_parameter_list
pub fn cmd_deps(
    paths: &[String],
    format: &DepsFormat,
    depth: &DepsDepth,
    graph_type: &DepsType,
    filter: Option<&str>,
    exact: bool,
    detail: bool,
    direction: &DepsDirection,
) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = crate::load_config(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);
    let mut cache = crate::open_project_cache(&cwd);

    let edges = match graph_type {
        DepsType::Imports => build_import_graph(&files, &cwd, depth),
        DepsType::Classes => build_class_graph(&files, &cwd, &mut cache),
        DepsType::Calls => build_call_graph(&files, &cwd, &mut cache),
    };

    let edges = apply_filter(edges, filter, exact, direction);
    let cycles = detect_cycles(&edges);
    let style = match graph_type {
        DepsType::Imports => CycleStyle::CircularDep,
        DepsType::Calls => CycleStyle::Recursion,
        DepsType::Classes => CycleStyle::CircularDep,
    };

    if detail && matches!(graph_type, DepsType::Classes) {
        let parsed = parse_all_models(&files, &cwd, &mut cache);
        render_detail_classes(&edges, &parsed, format, filter, exact);
    } else {
        render(&edges, &cycles, format, &style);
    }
    cache.flush();

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

fn apply_filter(
    edges: Vec<Edge>,
    filter: Option<&str>,
    exact: bool,
    direction: &DepsDirection,
) -> Vec<Edge> {
    let Some(pattern) = filter else {
        return edges;
    };
    let re = regex::Regex::new(pattern)
        .unwrap_or_else(|_| regex::Regex::new(&regex::escape(pattern)).unwrap());
    let matches = |s: &str| re.is_match(s);
    if exact {
        return edges
            .into_iter()
            .filter(|e| match direction {
                DepsDirection::Out => matches(&e.from),
                DepsDirection::In => matches(&e.to),
                DepsDirection::Both => matches(&e.from) || matches(&e.to),
            })
            .collect();
    }
    let matched = expand_connected(&edges, &re);
    edges
        .into_iter()
        .filter(|e| matched.contains(&e.from) && matched.contains(&e.to))
        .collect()
}

fn expand_connected(edges: &[Edge], re: &regex::Regex) -> HashSet<String> {
    let seeds: HashSet<String> = edges
        .iter()
        .filter(|e| re.is_match(&e.from) || re.is_match(&e.to))
        .flat_map(|e| [e.from.clone(), e.to.clone()])
        .collect();
    let mut matched = seeds.clone();
    expand_down(edges, &mut matched);
    expand_up(edges, &seeds, &mut matched);
    matched
}

fn expand_down(edges: &[Edge], matched: &mut HashSet<String>) {
    let mut changed = true;
    while changed {
        changed = false;
        for e in edges {
            if matched.contains(&e.from) && matched.insert(e.to.clone()) {
                changed = true;
            }
        }
    }
}

fn expand_up(edges: &[Edge], seeds: &HashSet<String>, matched: &mut HashSet<String>) {
    for seed in seeds {
        let mut current = seed.clone();
        let mut visited = HashSet::new();
        while visited.insert(current.clone()) {
            if let Some(e) = edges.iter().find(|e| e.from == current) {
                matched.insert(e.to.clone());
                current = e.to.clone();
            } else {
                break;
            }
        }
    }
}

// ── Import graph (existing) ──

const SKIP_PREFIXES: &[&str] = &["std::", "core::", "alloc::", "crate::", "super::", "self::"];

fn build_import_graph(files: &[PathBuf], cwd: &Path, depth: &DepsDepth) -> Vec<Edge> {
    let pb = crate::new_progress_bar(files.len() as u64);
    let mut edges = Vec::new();
    let mut cache = crate::open_project_cache(cwd);
    for path in files {
        let Some((src, model)) = crate::cached_parse(path, &mut cache, cwd) else {
            pb.inc(1);
            continue;
        };
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
    cache.flush();
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

pub(crate) fn parse_all_models(
    files: &[PathBuf],
    cwd: &std::path::Path,
    cache: &mut cha_core::ProjectCache,
) -> Vec<(PathBuf, cha_core::SourceModel)> {
    let pb = crate::new_progress_bar(files.len() as u64);
    let result = files
        .iter()
        .filter_map(|path| {
            let (_, model) = crate::cached_parse(path, cache, cwd)?;
            pb.inc(1);
            Some((path.clone(), model))
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
    fn from_files(files: &[PathBuf], models: &[&cha_core::SourceModel]) -> Self {
        let mut aliases = collect_typedef_aliases_from_models(models);
        for (k, v) in collect_typedef_aliases(files) {
            aliases.entry(k).or_insert(v);
        }
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

fn build_class_graph(
    files: &[PathBuf],
    cwd: &std::path::Path,
    cache: &mut cha_core::ProjectCache,
) -> Vec<Edge> {
    let parsed = parse_all_models(files, cwd, cache);
    let models: Vec<_> = parsed.iter().map(|(_, m)| m).collect();
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
fn collect_typedef_aliases_from_models(
    models: &[&cha_core::SourceModel],
) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for m in models {
        for (alias, original) in &m.type_aliases {
            aliases.insert(alias.clone(), original.clone());
        }
    }
    aliases
}

fn collect_typedef_aliases(files: &[PathBuf]) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for path in files {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        for line in content.lines() {
            let trimmed = line.trim();
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

fn build_call_graph(
    files: &[PathBuf],
    cwd: &std::path::Path,
    cache: &mut cha_core::ProjectCache,
) -> Vec<Edge> {
    let parsed = parse_all_models(files, cwd, cache);
    let known: HashSet<String> = parsed
        .iter()
        .flat_map(|(_, m)| &m.functions)
        .map(|f| f.name.clone())
        .collect();
    parsed
        .iter()
        .flat_map(|(_, m)| &m.functions)
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

struct DetailClass {
    name: String,
    fields: Vec<(String, String)>,
    methods: Vec<(String, bool)>, // (name, is_exported)
}

// cha:ignore long_method,high_complexity,brain_method,cognitive_complexity
fn render_detail_classes(
    edges: &[Edge],
    parsed: &[(PathBuf, cha_core::SourceModel)],
    format: &DepsFormat,
    filter: Option<&str>,
    exact: bool,
) {
    // Build alias maps from all models + file fallback
    let mut aliases: HashMap<String, String> = HashMap::new();
    for (path, m) in parsed {
        for (a, o) in &m.type_aliases {
            aliases.entry(a.clone()).or_insert(o.clone());
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if let Some(rest) = line.trim().strip_prefix("typedef struct ") {
                    let parts: Vec<&str> = rest.trim_end_matches(';').split_whitespace().collect();
                    if parts.len() == 2 {
                        aliases
                            .entry(parts[1].to_string())
                            .or_insert(parts[0].to_string());
                    }
                }
            }
        }
    }
    let reverse: HashMap<&str, &str> = aliases
        .iter()
        .map(|(a, o)| (o.as_str(), a.as_str()))
        .collect();
    let display = |name: &str| -> String { reverse.get(name).unwrap_or(&name).to_string() };

    // Determine which classes to show
    let mut edge_names: HashSet<String> = edges
        .iter()
        .flat_map(|e| {
            let mut v = vec![e.from.clone(), e.to.clone()];
            if let Some(o) = aliases.get(&e.from) {
                v.push(o.clone());
            }
            if let Some(o) = aliases.get(&e.to) {
                v.push(o.clone());
            }
            v
        })
        .collect();
    if exact && let Some(pattern) = filter {
        let re = regex::Regex::new(pattern)
            .unwrap_or_else(|_| regex::Regex::new(&regex::escape(pattern)).unwrap());
        edge_names.retain(|n| re.is_match(n) || re.is_match(&display(n)));
    }

    // Per-directory function index (for same-module matching)
    let mut dir_funcs: HashMap<&Path, Vec<&cha_core::FunctionInfo>> = HashMap::new();
    for (path, m) in parsed {
        let dir = path.parent().unwrap_or(path);
        dir_funcs.entry(dir).or_default().extend(&m.functions);
    }

    // Class → directory (prefer definition with fields over forward declarations)
    let mut class_dir: HashMap<&str, &Path> = HashMap::new();
    // First pass: only classes with fields (actual definitions)
    for (path, m) in parsed {
        let dir = path.parent().unwrap_or(path);
        for c in &m.classes {
            if c.field_count > 0 {
                class_dir.insert(&c.name, dir);
            }
        }
    }
    // Second pass: fill in any missing (forward declarations)
    for (path, m) in parsed {
        let dir = path.parent().unwrap_or(path);
        for c in &m.classes {
            class_dir.entry(&c.name).or_insert(dir);
        }
    }

    // Inheritance: class_name → parent_name
    let parent_map: HashMap<&str, &str> = parsed
        .iter()
        .flat_map(|(_, m)| &m.classes)
        .filter_map(|c| c.parent_name.as_deref().map(|p| (c.name.as_str(), p)))
        .collect();

    // Walk up inheritance chain to collect all ancestors (+ their aliases)
    let ancestors_of = |name: &str| -> HashSet<&str> {
        let mut set = HashSet::new();
        let mut cur = parent_map.get(name).copied();
        while let Some(p) = cur {
            set.insert(p);
            if let Some(&a) = reverse.get(p) {
                set.insert(a);
            }
            cur = parent_map.get(p).copied();
        }
        set
    };

    // Build detail classes
    let mut detail_classes: HashMap<String, DetailClass> = HashMap::new();
    for (_, m) in parsed {
        for c in &m.classes {
            if !edge_names.contains(&c.name) {
                continue;
            }
            let dn = display(&c.name);
            let existing_fields = detail_classes.get(&dn).map(|d| d.fields.len()).unwrap_or(0);
            if c.field_count < existing_fields {
                continue;
            }
            let fields: Vec<(String, String)> = c
                .field_names
                .iter()
                .zip(
                    c.field_types
                        .iter()
                        .chain(std::iter::repeat(&String::new())),
                )
                .map(|(n, t)| (n.clone(), t.clone()))
                .collect();

            let ancestors = ancestors_of(&c.name);
            let alias = reverse.get(c.name.as_str()).copied().unwrap_or(&c.name);

            // Same-module functions whose first param is this class or an ancestor
            let dir = class_dir.get(c.name.as_str()).or(class_dir.get(alias));
            let empty = vec![];
            let funcs = dir.and_then(|d| dir_funcs.get(d)).unwrap_or(&empty);

            let methods: Vec<(String, bool)> = m
                .functions
                .iter()
                .filter(|f| f.start_line >= c.start_line && f.end_line <= c.end_line)
                .chain(funcs.iter().copied().filter(|f| {
                    f.parameter_types.first().is_some_and(|t| {
                        if !t.raw.contains('*') {
                            return false;
                        }
                        let base = t.raw.split('*').next().unwrap_or("").trim();
                        base == c.name || base == alias || ancestors.contains(base)
                    })
                }))
                .map(|f| (f.name.clone(), f.is_exported))
                .collect();

            detail_classes.insert(
                dn,
                DetailClass {
                    name: display(&c.name),
                    fields,
                    methods,
                },
            );
        }
    }
    let classes: Vec<&DetailClass> = detail_classes.values().collect();

    match format {
        DepsFormat::Dot => render_detail_dot(&classes, edges),
        DepsFormat::Mermaid => render_detail_mermaid(&classes, edges),
        DepsFormat::Plantuml => render_detail_plantuml(&classes, edges),
        _ => render_detail_json(&classes, edges),
    }
}

fn render_detail_dot(classes: &[&DetailClass], edges: &[Edge]) {
    println!("digraph deps {{");
    println!("  rankdir=LR;");
    println!("  node [shape=record];");
    for c in classes {
        let fields: String = c
            .fields
            .iter()
            .map(|(n, t)| {
                if t.is_empty() {
                    format!("+ {n}\\l")
                } else {
                    format!("+ {n}: {t}\\l")
                }
            })
            .collect();
        let meths: String = c
            .methods
            .iter()
            .map(|(m, exported)| {
                let vis = if *exported { "+" } else { "-" };
                format!("{vis} {m}()\\l")
            })
            .collect();
        println!(
            "  \"{}\" [label=\"{{{}|{}|{}}}\"]; ",
            c.name, c.name, fields, meths
        );
    }
    for e in edges {
        println!("  \"{}\" -> \"{}\" [arrowhead=empty];", e.from, e.to);
    }
    println!("}}");
}

fn render_detail_mermaid(classes: &[&DetailClass], edges: &[Edge]) {
    println!("classDiagram");
    for c in classes {
        println!("    class {} {{", c.name);
        render_mermaid_members(c);
        println!("    }}");
    }
    for e in edges {
        let arrow = match e.label.as_deref() {
            Some("implements") => "..|>",
            _ => "--|>",
        };
        println!("    {} {} {}", e.from, arrow, e.to);
    }
}

fn render_mermaid_members(c: &DetailClass) {
    for (n, t) in &c.fields {
        if t.is_empty() {
            println!("        +{n}");
        } else {
            println!("        +{t} {n}");
        }
    }
    for (m, exported) in &c.methods {
        let vis = if *exported { "+" } else { "-" };
        println!("        {vis}{m}()");
    }
}

fn render_detail_json(classes: &[&DetailClass], edges: &[Edge]) {
    let nodes: Vec<serde_json::Value> = classes.iter().map(|c| {
        serde_json::json!({
            "name": c.name,
            "fields": c.fields.iter().map(|(n, t)| serde_json::json!({"name": n, "type": t})).collect::<Vec<_>>(),
            "methods": c.methods.iter().map(|(n, e)| serde_json::json!({"name": n, "exported": e})).collect::<Vec<_>>(),
        })
    }).collect();
    let json = serde_json::json!({
        "classes": nodes,
        "edges": edges.iter().map(|e| serde_json::json!({"from": e.from, "to": e.to, "label": e.label})).collect::<Vec<_>>(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );
}

fn render(edges: &[Edge], cycles: &[(String, String)], format: &DepsFormat, style: &CycleStyle) {
    match format {
        DepsFormat::Dot => print_dot(edges, cycles, style),
        DepsFormat::Json => print_json(edges, cycles),
        DepsFormat::Mermaid => print_mermaid(edges, cycles, style),
        DepsFormat::Plantuml => print_plantuml(edges, cycles),
        _ => print_dot(edges, cycles, style), // DSM/Terminal not applicable to deps
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

// ── PlantUML output ──

fn print_plantuml(edges: &[Edge], cycles: &[(String, String)]) {
    let cycle_set: HashSet<(&str, &str)> = cycles
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    println!("@startuml");
    for e in edges {
        let label = e.label.as_deref().unwrap_or("");
        let color = if cycle_set.contains(&(e.from.as_str(), e.to.as_str())) {
            " #red"
        } else {
            ""
        };
        if label.is_empty() {
            println!("  [{}] --> [{}]{}", e.from, e.to, color);
        } else {
            println!("  [{}] --> [{}]{} : {}", e.from, e.to, color, label);
        }
    }
    println!("@enduml");
}

// cha:ignore cognitive_complexity
fn render_detail_plantuml(classes: &[&DetailClass], edges: &[Edge]) {
    println!("@startuml");
    for c in classes {
        println!("class {} {{", c.name);
        for (name, ty) in &c.fields {
            if ty.is_empty() {
                println!("  +{name}");
            } else {
                println!("  +{name} : {ty}");
            }
        }
        for (name, is_exported) in &c.methods {
            let vis = if *is_exported { "+" } else { "-" };
            println!("  {vis}{name}()");
        }
        println!("}}");
    }
    for e in edges {
        let label = e.label.as_deref().unwrap_or("");
        if label.is_empty() {
            println!("{} --> {}", e.from, e.to);
        } else {
            println!("{} --> {} : {}", e.from, e.to, label);
        }
    }
    println!("@enduml");
}
