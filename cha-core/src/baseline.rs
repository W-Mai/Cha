use crate::Finding;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// A baseline is a set of finding fingerprints representing known/accepted issues.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Baseline {
    pub fingerprints: HashSet<String>,
}

impl Baseline {
    /// Build a baseline from a list of findings.
    pub fn from_findings(findings: &[Finding], project_root: &Path) -> Self {
        let fingerprints = findings
            .iter()
            .map(|f| fingerprint(f, project_root))
            .collect();
        Self { fingerprints }
    }

    /// Load from a JSON file.
    pub fn load(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save to a JSON file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, json)
    }

    /// Filter out findings that are already in the baseline.
    pub fn filter_new(&self, findings: Vec<Finding>, project_root: &Path) -> Vec<Finding> {
        findings
            .into_iter()
            .filter(|f| !self.fingerprints.contains(&fingerprint(f, project_root)))
            .collect()
    }
}

/// Stable fingerprint: smell_name + relative_path + symbol_name.
/// Deliberately excludes line numbers so that minor edits don't invalidate the baseline.
fn fingerprint(f: &Finding, project_root: &Path) -> String {
    let rel = f
        .location
        .path
        .strip_prefix(project_root)
        .unwrap_or(&f.location.path)
        .to_string_lossy();
    let symbol = f.location.name.as_deref().unwrap_or("");
    format!("{}:{}:{}", f.smell_name, rel, symbol)
}
