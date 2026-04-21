use crate::Finding;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Cache entry: content hash → findings.
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    content_hash: u64,
    findings: Vec<Finding>,
}

/// On-disk analysis cache stored at `.cha/cache/analysis.json`.
#[derive(Debug, Serialize, Deserialize, Default)]
struct CacheData {
    /// Hash of all config files + plugin versions; if changed, entire cache is invalid.
    env_hash: u64,
    entries: HashMap<String, CacheEntry>,
}

/// Incremental analysis cache.
///
/// Key = file path (relative), value = (content_hash, findings).
/// The entire cache is invalidated when `env_hash` changes
/// (config edits, plugin additions/removals).
pub struct AnalysisCache {
    path: PathBuf,
    data: CacheData,
    dirty: bool,
}

fn hash_all_configs(dir: &Path, h: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    let cfg = dir.join(".cha.toml");
    if let Ok(content) = std::fs::read_to_string(&cfg) {
        content.hash(h);
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            let s = name.to_string_lossy();
            if !s.starts_with('.') && !matches!(s.as_ref(), "target" | "node_modules" | "dist") {
                hash_all_configs(&path, h);
            }
        }
    }
}

impl AnalysisCache {
    /// Open (or create) a cache for the given project root.
    pub fn open(project_root: &Path, env_hash: u64) -> Self {
        let path = project_root.join(".cha/cache/analysis.json");
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<CacheData>(&s).ok())
            .unwrap_or_default();
        // Invalidate if environment changed.
        let data = if data.env_hash != env_hash {
            CacheData {
                env_hash,
                entries: HashMap::new(),
            }
        } else {
            data
        };
        Self {
            path,
            data,
            dirty: false,
        }
    }

    /// Look up cached findings for a file. Returns `Some` if content hash matches.
    pub fn get(&self, rel_path: &str, content_hash: u64) -> Option<&[Finding]> {
        let entry = self.data.entries.get(rel_path)?;
        if entry.content_hash == content_hash {
            Some(&entry.findings)
        } else {
            None
        }
    }

    /// Store findings for a file.
    pub fn put(&mut self, rel_path: String, content_hash: u64, findings: Vec<Finding>) {
        self.data.entries.insert(
            rel_path,
            CacheEntry {
                content_hash,
                findings,
            },
        );
        self.dirty = true;
    }

    /// Flush to disk if anything changed.
    pub fn flush(&self) {
        if !self.dirty {
            return;
        }
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string(&self.data) {
            let _ = std::fs::write(&self.path, json);
        }
    }

    /// Compute a content hash using the same fast hasher.
    pub fn hash_content(content: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut h);
        h.finish()
    }

    /// Compute an environment hash from config content + plugin file mtimes.
    pub fn env_hash(project_root: &Path, plugin_dirs: &[PathBuf]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        // Invalidate when cha version changes (plugin logic may differ)
        env!("CARGO_PKG_VERSION").hash(&mut h);
        // Hash all .cha.toml files (root + subdirectories)
        hash_all_configs(project_root, &mut h);
        for dir in plugin_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
                        mtime.hash(&mut h);
                    }
                    entry.file_name().hash(&mut h);
                }
            }
        }
        h.finish()
    }
}
