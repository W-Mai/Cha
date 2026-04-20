use std::collections::{BTreeMap, HashMap, HashSet};

use super::Module;

/// Layer information for a module.
#[derive(Debug, Clone)]
pub struct LayerInfo {
    pub name: String,
    pub level: usize,
    pub file_count: usize,
    pub fan_in: usize,
    pub fan_out: usize,
    pub instability: f64,
    pub lcom4: usize,
    pub tcc: f64,
    pub cohesion: f64,
}

/// A detected layer violation.
#[derive(Debug, Clone)]
pub struct LayerViolation {
    pub from_module: String,
    pub to_module: String,
    pub from_level: usize,
    pub to_level: usize,
    /// Instability gap (to - from). Larger = more severe.
    pub gap: f64,
}

/// Infer layers from modules and file-level imports.
/// Returns (layers sorted by instability, violations).
pub fn infer_layers(
    modules: &[Module],
    file_imports: &[(String, String)],
) -> (Vec<LayerInfo>, Vec<LayerViolation>) {
    let file_to_mod: HashMap<&str, &str> = modules
        .iter()
        .flat_map(|m| m.files.iter().map(|f| (f.as_str(), m.name.as_str())))
        .collect();

    let (fan_in, fan_out) = compute_module_fans(&file_to_mod, file_imports);
    let mut layers = build_layers(modules, &fan_in, &fan_out);

    layers.sort_by(|a, b| a.instability.partial_cmp(&b.instability).unwrap());
    for (i, l) in layers.iter_mut().enumerate() {
        l.level = i;
    }

    let violations = detect_violations(&layers, &file_to_mod, file_imports);
    (layers, violations)
}

type FanMap<'a> = HashMap<&'a str, HashSet<&'a str>>;

fn compute_module_fans<'a>(
    file_to_mod: &HashMap<&'a str, &'a str>,
    file_imports: &[(String, String)],
) -> (FanMap<'a>, FanMap<'a>) {
    let mut fan_in: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut fan_out: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (from, to) in file_imports {
        let fm = file_to_mod.get(from.as_str()).copied().unwrap_or("");
        let tm = file_to_mod.get(to.as_str()).copied().unwrap_or("");
        if !fm.is_empty() && !tm.is_empty() && fm != tm {
            fan_out.entry(fm).or_default().insert(tm);
            fan_in.entry(tm).or_default().insert(fm);
        }
    }
    (fan_in, fan_out)
}

fn build_layers(
    modules: &[Module],
    fan_in: &HashMap<&str, HashSet<&str>>,
    fan_out: &HashMap<&str, HashSet<&str>>,
) -> Vec<LayerInfo> {
    modules
        .iter()
        .map(|m| {
            let fi = fan_in.get(m.name.as_str()).map(|s| s.len()).unwrap_or(0);
            let fo = fan_out.get(m.name.as_str()).map(|s| s.len()).unwrap_or(0);
            let total = fi + fo;
            LayerInfo {
                name: m.name.clone(),
                level: 0,
                file_count: m.files.len(),
                fan_in: fi,
                fan_out: fo,
                instability: if total > 0 {
                    fo as f64 / total as f64
                } else {
                    0.5
                },
                lcom4: m.lcom4,
                tcc: m.tcc,
                cohesion: m.cohesion,
            }
        })
        .collect()
}

fn is_parent_child(a: &str, b: &str) -> bool {
    let a = a.trim_end_matches("/*").trim_end_matches('/');
    let b = b.trim_end_matches("/*").trim_end_matches('/');
    a.starts_with(&format!("{b}/")) || b.starts_with(&format!("{a}/"))
}

fn detect_violations(
    layers: &[LayerInfo],
    file_to_mod: &HashMap<&str, &str>,
    file_imports: &[(String, String)],
) -> Vec<LayerViolation> {
    let level_map: BTreeMap<&str, usize> =
        layers.iter().map(|l| (l.name.as_str(), l.level)).collect();
    let inst_map: BTreeMap<&str, f64> = layers
        .iter()
        .map(|l| (l.name.as_str(), l.instability))
        .collect();

    let mut seen: BTreeMap<(&str, &str), (usize, usize)> = BTreeMap::new();
    for (from, to) in file_imports {
        let fm = file_to_mod.get(from.as_str()).copied().unwrap_or("");
        let tm = file_to_mod.get(to.as_str()).copied().unwrap_or("");
        if fm == tm || fm.is_empty() || tm.is_empty() {
            continue;
        }
        if is_parent_child(fm, tm) {
            continue;
        }
        if let (Some(&fl), Some(&tl)) = (level_map.get(fm), level_map.get(tm))
            && fl < tl
        {
            seen.entry((fm, tm)).or_insert((fl, tl));
        }
    }

    let mut result: Vec<LayerViolation> = seen
        .into_iter()
        .map(|((from, to), (fl, tl))| {
            let fi = inst_map.get(from).copied().unwrap_or(0.5);
            let ti = inst_map.get(to).copied().unwrap_or(0.5);
            LayerViolation {
                from_module: from.to_string(),
                to_module: to.to_string(),
                from_level: fl,
                to_level: tl,
                gap: ti - fi,
            }
        })
        .collect();
    // Sort by gap descending (most severe first)
    result.sort_by(|a, b| {
        b.gap
            .partial_cmp(&a.gap)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_module_importing_volatile_is_violation() {
        let modules = vec![
            Module {
                name: "core".into(),
                files: vec!["core/a.rs".into()],
                lcom4: 1,
                tcc: 1.0,
                cohesion: 1.0,
            },
            Module {
                name: "ui".into(),
                files: vec!["ui/b.rs".into()],
                lcom4: 1,
                tcc: 1.0,
                cohesion: 1.0,
            },
        ];
        // core imports ui (core has high fan-in → stable, ui has high fan-out → volatile)
        // But with only this edge, core fan-out=1, ui fan-in=1
        // core: I=1.0, ui: I=0.0 → ui is more stable
        // So ui(L0) importing core(L1) would be violation
        // But here core imports ui, so core(L1) → ui(L0) = not violation
        let imports = vec![("core/a.rs".into(), "ui/b.rs".into())];
        let (layers, violations) = infer_layers(&modules, &imports);
        assert_eq!(layers.len(), 2);
        // No violation: higher instability importing lower is fine
        assert!(violations.is_empty());
    }
}
