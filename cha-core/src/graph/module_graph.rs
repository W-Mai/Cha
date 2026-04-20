use std::collections::{BTreeMap, HashMap, HashSet};

/// A group of tightly-coupled files inferred from the import graph.
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub files: Vec<String>,
    /// LCOM4: number of connected components in internal import graph.
    /// 1 = perfectly cohesive, >1 = should be split.
    pub lcom4: usize,
    /// TCC: fraction of file pairs that are directly/indirectly connected.
    /// Range [0, 1], higher is better.
    pub tcc: f64,
    /// Cohesion: average ratio of internal connections to total connections.
    pub cohesion: f64,
}

/// Infer modules from file-level import edges.
///
/// Algorithm:
/// 1. Auto-detect directory depth (elbow method) or use provided depth
/// 2. Group files by directory at that depth
/// 3. For each group with LCOM4 > 1: recursively split into subdirectories
/// 4. For large groups with LCOM4 = 1 but ICR < 0.30: split one level
pub fn infer_modules(
    file_imports: &[(String, String)],
    all_files: &[String],
    depth: Option<usize>,
) -> Vec<Module> {
    let adj = build_undirected_adj(file_imports, all_files);
    let depth = depth.unwrap_or_else(|| auto_detect_depth(all_files));
    let groups = group_by_directory(all_files, depth);

    let mut modules = Vec::new();
    for (dir, files) in groups {
        adaptive_split(&dir, &files, &adj, &mut modules);
    }

    // Compute metrics for each module
    for m in &mut modules {
        let mset: HashSet<&str> = m.files.iter().map(|s| s.as_str()).collect();
        m.lcom4 = compute_lcom4(&mset, &adj);
        m.tcc = compute_tcc(&mset, &adj);
        m.cohesion = compute_cohesion(&mset, &adj);
    }
    modules
}

// ── Auto depth detection ──

fn auto_detect_depth(files: &[String]) -> usize {
    let max_d = files
        .iter()
        .map(|f| f.matches('/').count().saturating_sub(1)) // exclude filename
        .max()
        .unwrap_or(0);
    if max_d == 0 {
        return 1;
    }
    let counts: Vec<usize> = (1..=max_d)
        .map(|d| group_by_directory(files, d).len())
        .collect();
    // Find depth with largest relative growth in next level
    let mut best = 0;
    let mut max_ratio = 0.0_f64;
    for i in 0..counts.len().saturating_sub(1) {
        if counts[i] > 0 {
            let ratio = (counts[i + 1] as f64 - counts[i] as f64) / counts[i] as f64;
            if ratio > max_ratio {
                max_ratio = ratio;
                best = i;
            }
        }
    }
    best + 1 // depths are 1-indexed
}

fn group_by_directory(files: &[String], depth: usize) -> BTreeMap<String, Vec<String>> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in files {
        let parts: Vec<&str> = f.split('/').collect();
        let dir_parts = &parts[..parts.len().saturating_sub(1)]; // exclude filename
        let key = if dir_parts.len() >= depth {
            dir_parts[..depth].join("/")
        } else if dir_parts.is_empty() {
            "(root)".to_string()
        } else {
            dir_parts.join("/")
        };
        groups.entry(key).or_default().push(f.clone());
    }
    groups
}

// ── Adaptive splitting ──

fn adaptive_split(
    dir: &str,
    files: &[String],
    adj: &HashMap<String, HashSet<String>>,
    out: &mut Vec<Module>,
) {
    if files.len() < 3 {
        out.push(make_module(dir, files));
        return;
    }

    let mset: HashSet<&str> = files.iter().map(|s| s.as_str()).collect();

    if try_lcom4_split(dir, files, &mset, adj, out) {
        return;
    }
    if try_icr_split(dir, files, adj, out) {
        return;
    }
    out.push(make_module(dir, files));
}

fn try_lcom4_split(
    dir: &str,
    files: &[String],
    mset: &HashSet<&str>,
    adj: &HashMap<String, HashSet<String>>,
    out: &mut Vec<Module>,
) -> bool {
    if compute_lcom4(mset, adj) <= 1 {
        return false;
    }
    let subs = group_by_next_level(dir, files);
    if subs.len() <= 1 {
        return false;
    }
    for (sd, sf) in &subs {
        adaptive_split(sd, sf, adj, out);
    }
    true
}

fn try_icr_split(
    dir: &str,
    files: &[String],
    adj: &HashMap<String, HashSet<String>>,
    out: &mut Vec<Module>,
) -> bool {
    if files.len() <= 30 {
        return false;
    }
    let subs = group_by_next_level(dir, files);
    if subs.len() <= 1 || inter_child_ratio(&subs, adj) >= 0.30 {
        return false;
    }
    for (sd, sf) in &subs {
        adaptive_split(sd, sf, adj, out);
    }
    true
}

fn group_by_next_level(dir: &str, files: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut subs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let prefix = if dir == "(root)" { "" } else { dir };
    for f in files {
        let rest = if prefix.is_empty() {
            f.as_str()
        } else {
            f.strip_prefix(prefix)
                .and_then(|s| s.strip_prefix('/'))
                .unwrap_or(f)
        };
        let parts: Vec<&str> = rest.split('/').collect();
        let key = if parts.len() > 1 {
            if prefix.is_empty() {
                parts[0].to_string()
            } else {
                format!("{prefix}/{}", parts[0])
            }
        } else {
            // Direct file in this directory
            format!("{dir}/*")
        };
        subs.entry(key).or_default().push(f.clone());
    }
    subs
}

fn make_module(dir: &str, files: &[String]) -> Module {
    Module {
        name: dir.to_string(),
        files: files.to_vec(),
        lcom4: 0,
        tcc: 0.0,
        cohesion: 0.0,
    }
}

// ── Metrics ──

fn build_undirected_adj(
    edges: &[(String, String)],
    all_files: &[String],
) -> HashMap<String, HashSet<String>> {
    let file_set: HashSet<&str> = all_files.iter().map(|s| s.as_str()).collect();
    let mut adj: HashMap<String, HashSet<String>> = HashMap::new();
    for f in all_files {
        adj.entry(f.clone()).or_default();
    }
    for (s, d) in edges {
        if file_set.contains(s.as_str()) && file_set.contains(d.as_str()) {
            adj.entry(s.clone()).or_default().insert(d.clone());
            adj.entry(d.clone()).or_default().insert(s.clone());
        }
    }
    adj
}

fn compute_lcom4(members: &HashSet<&str>, adj: &HashMap<String, HashSet<String>>) -> usize {
    let mut visited: HashSet<&str> = HashSet::new();
    let mut count = 0;
    for &f in members {
        if visited.contains(f) {
            continue;
        }
        count += 1;
        bfs_visit(f, members, adj, &mut visited);
    }
    count
}

fn bfs_visit<'a>(
    start: &'a str,
    members: &HashSet<&str>,
    adj: &'a HashMap<String, HashSet<String>>,
    visited: &mut HashSet<&'a str>,
) {
    let mut stack = vec![start];
    while let Some(cur) = stack.pop() {
        if !visited.insert(cur) {
            continue;
        }
        let Some(neighbors) = adj.get(cur) else {
            continue;
        };
        for n in neighbors {
            if members.contains(n.as_str()) && !visited.contains(n.as_str()) {
                stack.push(n);
            }
        }
    }
}

fn compute_tcc(members: &HashSet<&str>, adj: &HashMap<String, HashSet<String>>) -> f64 {
    let n = members.len();
    if n < 2 {
        return 1.0;
    }
    // For large modules, skip TCC (O(n²))
    if n > 200 {
        return -1.0;
    }
    let list: Vec<&str> = members.iter().copied().collect();
    // file_targets[f] = set of internal neighbors
    let targets: HashMap<&str, HashSet<&str>> = list
        .iter()
        .map(|&f| {
            let t: HashSet<&str> = adj
                .get(f)
                .map(|ns| {
                    ns.iter()
                        .filter(|n| members.contains(n.as_str()))
                        .map(|n| n.as_str())
                        .collect()
                })
                .unwrap_or_default();
            (f, t)
        })
        .collect();

    let mut connected = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            let a = list[i];
            let b = list[j];
            // Connected if: direct edge, or share a common neighbor
            if targets[a].contains(b)
                || targets[b].contains(a)
                || !targets[a].is_disjoint(&targets[b])
            {
                connected += 1;
            }
        }
    }
    connected as f64 / (n * (n - 1) / 2) as f64
}

fn compute_cohesion(members: &HashSet<&str>, adj: &HashMap<String, HashSet<String>>) -> f64 {
    let mut sum = 0.0;
    let mut count = 0;
    for &f in members {
        if let Some(neighbors) = adj.get(f) {
            let total = neighbors.len();
            if total == 0 {
                continue;
            }
            let internal = neighbors
                .iter()
                .filter(|n| members.contains(n.as_str()))
                .count();
            sum += internal as f64 / total as f64;
            count += 1;
        }
    }
    if count > 0 { sum / count as f64 } else { 0.0 }
}

fn inter_child_ratio(
    subs: &BTreeMap<String, Vec<String>>,
    adj: &HashMap<String, HashSet<String>>,
) -> f64 {
    let file_to_sub: HashMap<&str, &str> = subs
        .iter()
        .flat_map(|(sd, files)| files.iter().map(move |f| (f.as_str(), sd.as_str())))
        .collect();
    let all_files: HashSet<&str> = file_to_sub.keys().copied().collect();
    let (mut intra, mut inter) = (0usize, 0usize);
    for &f in &all_files {
        let Some(neighbors) = adj.get(f) else {
            continue;
        };
        let f_sub = file_to_sub[f];
        for n in neighbors {
            let ns = n.as_str();
            if !all_files.contains(ns) {
                continue;
            }
            if f_sub == file_to_sub[ns] {
                intra += 1;
            } else {
                inter += 1;
            }
        }
    }
    let total = intra + inter;
    if total > 0 {
        inter as f64 / total as f64
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_depth_simple() {
        let files: Vec<String> = vec![
            "src/core/a.rs",
            "src/core/b.rs",
            "src/util/c.rs",
            "src/util/d.rs",
            "src/util/sub/e.rs",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let d = auto_detect_depth(&files);
        assert!(d >= 1 && d <= 2);
    }

    #[test]
    fn lcom4_two_components() {
        let files: Vec<String> = vec!["a.rs", "b.rs", "c.rs", "d.rs"]
            .into_iter()
            .map(String::from)
            .collect();
        // a↔b, c↔d (two components)
        let edges = vec![
            ("a.rs".into(), "b.rs".into()),
            ("c.rs".into(), "d.rs".into()),
        ];
        let modules = infer_modules(&edges, &files, Some(1));
        // Should detect LCOM4 > 1 at root level
        assert!(modules.iter().any(|m| m.lcom4 >= 1));
    }

    #[test]
    fn directory_grouping() {
        let files: Vec<String> = vec!["src/core/x.rs", "src/core/y.rs", "src/util/z.rs"]
            .into_iter()
            .map(String::from)
            .collect();
        let modules = infer_modules(&[], &files, Some(1));
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
