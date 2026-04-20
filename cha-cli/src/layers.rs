use std::collections::HashMap;
use std::path::PathBuf;

use cha_core::SourceFile;
use cha_core::graph;

use crate::{DepsFormat, analyze::filter_excluded, collect_files};

pub fn cmd_layers(paths: &[String], save: bool, format: &DepsFormat, depth: Option<usize>) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = crate::load_config(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);

    let (file_imports, all_files) = build_import_edges(&files, &cwd);

    let (modules, layers, violations) =
        if !root_config.layers.modules.is_empty() && !root_config.layers.tiers.is_empty() {
            manual_layers(&root_config.layers, &all_files, &file_imports, &cwd)
        } else {
            let modules = graph::infer_modules(&file_imports, &all_files, depth);
            let (layers, violations) = graph::infer_layers(&modules, &file_imports);
            (modules, layers, violations)
        };

    match format {
        DepsFormat::Dot => render_dot(&layers, &violations),
        DepsFormat::Mermaid => render_mermaid(&layers, &violations),
        DepsFormat::Json => render_json(&layers, &violations),
        DepsFormat::Plantuml => render_plantuml(&layers, &violations),
        DepsFormat::Dsm => render_dsm(&layers, &file_imports, &modules),
        DepsFormat::Terminal => render_terminal(&layers, &violations),
        DepsFormat::Html => render_html(&layers, &violations),
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

fn manual_layers(
    cfg: &cha_core::LayersConfig,
    all_files: &[String],
    file_imports: &[(String, String)],
    cwd: &std::path::Path,
) -> (
    Vec<graph::Module>,
    Vec<graph::LayerInfo>,
    Vec<graph::LayerViolation>,
) {
    let (modules, file_to_mod) = manual_modules(cfg, all_files, cwd);
    let layers = manual_layer_info(cfg, &modules);
    let violations = manual_violations(cfg, file_imports, &file_to_mod, &layers);
    (modules, layers, violations)
}

fn manual_modules(
    cfg: &cha_core::LayersConfig,
    all_files: &[String],
    cwd: &std::path::Path,
) -> (Vec<graph::Module>, HashMap<String, String>) {
    let mut modules = Vec::new();
    let mut f2m: HashMap<String, String> = HashMap::new();
    for (name, patterns) in &cfg.modules {
        let mut matched = Vec::new();
        for f in all_files {
            let full = cwd.join(f);
            for pat in patterns {
                let ok = glob::glob(&cwd.join(pat).to_string_lossy())
                    .into_iter()
                    .flatten()
                    .any(|p| p.as_ref().map(|p| p == &full).unwrap_or(false));
                if ok {
                    matched.push(f.clone());
                    f2m.insert(f.clone(), name.clone());
                    break;
                }
            }
        }
        modules.push(graph::Module {
            name: name.clone(),
            files: matched,
            lcom4: 0,
            tcc: 0.0,
            cohesion: 0.0,
        });
    }
    (modules, f2m)
}

fn manual_layer_info(
    cfg: &cha_core::LayersConfig,
    modules: &[graph::Module],
) -> Vec<graph::LayerInfo> {
    let mod_map: HashMap<&str, &graph::Module> =
        modules.iter().map(|m| (m.name.as_str(), m)).collect();
    let mut layers = Vec::new();
    for (level, tier) in cfg.tiers.iter().enumerate() {
        for mod_name in &tier.modules {
            if let Some(m) = mod_map.get(mod_name.as_str()) {
                layers.push(graph::LayerInfo {
                    name: mod_name.clone(),
                    level,
                    file_count: m.files.len(),
                    fan_in: 0,
                    fan_out: 0,
                    instability: level as f64 / cfg.tiers.len().max(1) as f64,
                    lcom4: 0,
                    tcc: 0.0,
                    cohesion: 0.0,
                });
            }
        }
    }
    layers
}

fn manual_violations(
    cfg: &cha_core::LayersConfig,
    file_imports: &[(String, String)],
    f2m: &HashMap<String, String>,
    layers: &[graph::LayerInfo],
) -> Vec<graph::LayerViolation> {
    let mod_level: HashMap<&str, usize> =
        layers.iter().map(|l| (l.name.as_str(), l.level)).collect();
    let mut seen: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
    let mut violations = Vec::new();
    for (from, to) in file_imports {
        let fm = f2m.get(from).map(|s| s.as_str()).unwrap_or("");
        let tm = f2m.get(to).map(|s| s.as_str()).unwrap_or("");
        if fm.is_empty() || tm.is_empty() || fm == tm {
            continue;
        }
        let fl = mod_level.get(fm).copied().unwrap_or(0);
        let tl = mod_level.get(tm).copied().unwrap_or(0);
        if fl < tl && seen.insert((fm.to_string(), tm.to_string())) {
            violations.push(graph::LayerViolation {
                from_module: fm.to_string(),
                to_module: tm.to_string(),
                from_level: fl,
                to_level: tl,
                gap: (tl as f64 - fl as f64) / cfg.tiers.len().max(1) as f64,
                evidence: vec![(from.clone(), to.clone())],
            });
        }
    }
    violations
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
    let all_names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
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
            let short = short_module_name(&l.name, &all_names);
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
    let all_names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
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
    render_dot_bands(&shown, &all_names);
    for v in violations {
        println!(
            "  {:?} -> {:?} [color=red penwidth=2];",
            v.from_module, v.to_module
        );
    }
    println!("}}");
}

fn render_dot_bands(shown: &[&graph::LayerInfo], all_names: &[&str]) {
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
            let short = short_module_name(&l.name, all_names);
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
                "fan_in": l.fan_in, "fan_out": l.fan_out, "instability": l.instability,
                "lcom4": l.lcom4, "tcc": l.tcc, "cohesion": l.cohesion
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

fn short_module_name(full: &str, all: &[&str]) -> String {
    let prefix = common_path_prefix(all);
    let rel = full
        .strip_prefix(&prefix)
        .unwrap_or(full)
        .trim_start_matches('/')
        .trim_end_matches("/*")
        .trim_end_matches('/');
    if rel.is_empty() {
        return "(root)".to_string();
    }
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() >= 2 {
        format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        parts[0].to_string()
    }
}

fn common_path_prefix(names: &[&str]) -> String {
    if names.is_empty() {
        return String::new();
    }
    let parts: Vec<Vec<&str>> = names.iter().map(|n| n.split('/').collect()).collect();
    let prefix: Vec<&str> = (0..)
        .map_while(|i| {
            let first = parts[0].get(i)?;
            parts
                .iter()
                .all(|p| p.get(i) == Some(first))
                .then_some(*first)
        })
        .collect();
    prefix.join("/")
}

fn render_dsm(
    layers: &[graph::LayerInfo],
    file_imports: &[(String, String)],
    modules: &[graph::Module],
) {
    let mut shown: Vec<&graph::LayerInfo> =
        layers.iter().filter(|l| l.fan_in + l.fan_out > 0).collect();
    if shown.is_empty() {
        println!("No cross-module dependencies found.");
        return;
    }
    shown.sort_by_key(|l| std::cmp::Reverse(l.file_count));
    shown.truncate(25);
    shown.sort_by(|a, b| {
        a.instability
            .partial_cmp(&b.instability)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let f2m = dsm_file_to_mod(modules);
    let ec = dsm_edge_counts(file_imports, &f2m);
    let names: Vec<&str> = shown.iter().map(|l| l.name.as_str()).collect();
    let short: Vec<String> = names
        .iter()
        .map(|n| {
            let s = short_module_name(n, &names);
            s.chars().take(8).collect()
        })
        .collect();
    dsm_print(&names, &short, &ec);
}

fn dsm_file_to_mod(modules: &[graph::Module]) -> HashMap<&str, &str> {
    modules
        .iter()
        .flat_map(|m| m.files.iter().map(move |f| (f.as_str(), m.name.as_str())))
        .collect()
}

fn dsm_edge_counts<'a>(
    imports: &[(String, String)],
    f2m: &HashMap<&'a str, &'a str>,
) -> HashMap<(&'a str, &'a str), usize> {
    let mut ec = HashMap::new();
    for (from, to) in imports {
        let fm = f2m.get(from.as_str()).copied().unwrap_or("");
        let tm = f2m.get(to.as_str()).copied().unwrap_or("");
        if !fm.is_empty() && !tm.is_empty() && fm != tm {
            *ec.entry((fm, tm)).or_default() += 1;
        }
    }
    ec
}

fn dsm_print(names: &[&str], short: &[String], ec: &HashMap<(&str, &str), usize>) {
    let w = 10;
    print!("{:>w$}", "");
    for s in short {
        print!(" {:>5}", &s[..s.len().min(5)]);
    }
    println!();
    for (i, &from) in names.iter().enumerate() {
        print!("{:>w$}", &short[i]);
        for (j, &to) in names.iter().enumerate() {
            if i == j {
                print!("    ██");
            } else {
                let c = ec.get(&(from, to)).copied().unwrap_or(0);
                if c == 0 {
                    print!("     ·");
                } else {
                    print!(" {:>5}", c);
                }
            }
        }
        println!();
    }
}

fn render_terminal(layers: &[graph::LayerInfo], violations: &[graph::LayerViolation]) {
    let all_names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
    println!(
        "Modules: {}, Violations: {}\n",
        layers.len(),
        violations.len()
    );
    render_terminal_bands(layers, &all_names);
    render_terminal_violations(violations, &all_names);
}

fn render_terminal_bands(layers: &[graph::LayerInfo], all_names: &[&str]) {
    const BANDS: &[(&str, f64, f64)] = &[
        ("🟢 Stable (I<0.2)", 0.0, 0.2),
        ("🔵 Core (0.2≤I<0.4)", 0.2, 0.4),
        ("🟡 Mid (0.4≤I<0.6)", 0.4, 0.6),
        ("🟠 Volatile (0.6≤I<0.8)", 0.6, 0.8),
        ("🔴 Leaf (I≥0.8)", 0.8, 1.01),
    ];
    for &(label, lo, hi) in BANDS {
        let members: Vec<&graph::LayerInfo> = layers
            .iter()
            .filter(|l| l.instability >= lo && l.instability < hi && l.file_count >= 3)
            .collect();
        if members.is_empty() {
            continue;
        }
        println!("  {label}");
        for l in &members {
            let short = short_module_name(&l.name, all_names);
            let tcc = if l.tcc >= 0.0 {
                format!("{:.0}%", l.tcc * 100.0)
            } else {
                "n/a".into()
            };
            let lcom = if l.lcom4 == 1 {
                "✓".into()
            } else {
                format!("⚠{}", l.lcom4)
            };
            println!(
                "    {:<30} {:>4}f  I={:.2}  TCC={:>4}  {}",
                short, l.file_count, l.instability, tcc, lcom
            );
        }
        println!();
    }
}

fn render_terminal_violations(violations: &[graph::LayerViolation], all_names: &[&str]) {
    if violations.is_empty() {
        return;
    }
    println!("  ⚡ Violations (stable → volatile):");
    for v in violations.iter().take(10) {
        let f = short_module_name(&v.from_module, all_names);
        let t = short_module_name(&v.to_module, all_names);
        println!("    {f} → {t}  (gap={:.2})", v.gap);
        for (src, dst) in v.evidence.iter().take(3) {
            let sf = src.split('/').next_back().unwrap_or(src);
            let sd = dst.split('/').next_back().unwrap_or(dst);
            println!("      {sf} includes {sd}");
        }
        if v.evidence.len() > 3 {
            println!("      ... {} more imports", v.evidence.len() - 3);
        }
    }
    if violations.len() > 10 {
        println!("    ... and {} more violations", violations.len() - 10);
    }
}

fn render_html(layers: &[graph::LayerInfo], violations: &[graph::LayerViolation]) {
    let all_names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
    let rows = html_tier_rows(layers, &all_names);
    let viols = html_violations(violations, &all_names);
    println!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Architecture</title>\
<style>*{{margin:0;padding:0;box-sizing:border-box}}\
body{{font-family:-apple-system,sans-serif;background:#f5f5f5;padding:20px}}\
h1{{text-align:center;font-size:20px;margin-bottom:4px}}\
.sub{{text-align:center;color:#666;font-size:14px;margin-bottom:20px}}\
.tier{{margin-bottom:12px;padding:12px 16px;border-radius:8px}}\
.tl{{font-weight:bold;font-size:15px;margin-bottom:8px}}.tl span{{font-weight:normal;font-size:12px}}\
.groups{{display:flex;flex-wrap:wrap;gap:10px}}\
.group{{background:#fff;border:1px solid #ddd;border-radius:6px;padding:8px 10px;min-width:120px}}\
.gt{{font-weight:600;font-size:13px;color:#333;margin-bottom:4px;border-bottom:1px solid #eee;padding-bottom:3px}}\
.mods{{display:flex;flex-wrap:wrap;gap:4px}}\
.mod{{background:#f0f0f0;border-radius:4px;padding:2px 8px;font-size:12px;color:#444;white-space:nowrap}}\
.fc{{color:#999;margin-left:4px;font-size:11px}}\
.vbox{{margin-top:16px;padding:12px;background:#fff;border-radius:8px;border-left:4px solid #f44336}}\
.vbox h3{{color:#f44336;font-size:14px;margin-bottom:6px}}\
.v{{font-size:12px;color:#666;padding:2px 0}}.v b{{color:#333}}\
</style></head><body>\
<h1>Architecture</h1><div class=\"sub\">{} modules · {} violations</div>\
{rows}<div class=\"vbox\"><h3>⚡ Layer Violations</h3>{viols}</div></body></html>",
        layers.len(),
        violations.len()
    );
}

const HTML_TIERS: &[(&str, f64, f64, &str, &str)] = &[
    ("Leaf", 0.8, 1.01, "#f44336", "#ffebee"),
    ("Volatile", 0.6, 0.8, "#ff9800", "#fff3e0"),
    ("Mid", 0.4, 0.6, "#ffc107", "#fffde7"),
    ("Core", 0.2, 0.4, "#2196f3", "#e3f2fd"),
    ("Stable", 0.0, 0.2, "#4caf50", "#e8f5e9"),
];

fn html_tier_rows(layers: &[graph::LayerInfo], all_names: &[&str]) -> String {
    let mut tier_groups: std::collections::BTreeMap<
        &str,
        std::collections::BTreeMap<String, Vec<&graph::LayerInfo>>,
    > = std::collections::BTreeMap::new();
    for l in layers {
        if l.file_count < 5 {
            continue;
        }
        let Some((tname, ..)) = HTML_TIERS
            .iter()
            .find(|(_, lo, hi, _, _)| l.instability >= *lo && l.instability < *hi)
        else {
            continue;
        };
        let sn = short_module_name(&l.name, all_names);
        let group = sn.split('/').next().unwrap_or(&sn).to_string();
        tier_groups
            .entry(tname)
            .or_default()
            .entry(group)
            .or_default()
            .push(l);
    }
    let mut rows = String::new();
    for &(tname, lo, hi, accent, bg) in HTML_TIERS {
        let Some(groups) = tier_groups.get(tname) else {
            continue;
        };
        let mut sorted: Vec<_> = groups.iter().collect();
        sorted.sort_by_key(|(_, ms)| {
            std::cmp::Reverse(ms.iter().map(|m| m.file_count).sum::<usize>())
        });
        let mut gh = String::new();
        for (gname, mods) in &sorted {
            let mh: String = mods
                .iter()
                .map(|m| {
                    let sn = short_module_name(&m.name, all_names);
                    format!(
                        "<div class=\"mod\">{sn}<span class=\"fc\">{}f</span></div>",
                        m.file_count
                    )
                })
                .collect();
            gh.push_str(&format!("<div class=\"group\"><div class=\"gt\">{gname}</div><div class=\"mods\">{mh}</div></div>"));
        }
        rows.push_str(&format!(
            "<div class=\"tier\" style=\"border-left:4px solid {accent};background:{bg}\">\
             <div class=\"tl\" style=\"color:{accent}\">{tname} <span>I={lo:.1}–{hi:.1}</span></div>\
             <div class=\"groups\">{gh}</div></div>"
        ));
    }
    rows
}

fn html_violations(violations: &[graph::LayerViolation], all_names: &[&str]) -> String {
    let mut sorted: Vec<_> = violations.iter().collect();
    sorted.sort_by_key(|v| std::cmp::Reverse(v.to_level.saturating_sub(v.from_level)));
    let mut s = String::new();
    for v in sorted.iter().take(15) {
        let f = short_module_name(&v.from_module, all_names);
        let t = short_module_name(&v.to_module, all_names);
        s.push_str(&format!("<div class=\"v\"><b>{f}</b> → <b>{t}</b></div>"));
    }
    if violations.len() > 15 {
        s.push_str(&format!(
            "<div class=\"v\" style=\"color:#999\">... and {} more</div>",
            violations.len() - 15
        ));
    }
    s
}
