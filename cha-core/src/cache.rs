use crate::{Finding, SourceModel, SymbolIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Per-file cache metadata.
#[derive(Debug, Serialize, Deserialize)]
struct FileEntry {
    mtime_secs: u64,
    size: u64,
    content_hash: u64,
    /// Cached import sources for fast unstable_dependency analysis.
    #[serde(default)]
    imports: Vec<String>,
}

/// Per-file findings cache entry.
#[derive(Debug, Serialize, Deserialize)]
struct FindingsEntry {
    content_hash: u64,
    findings: Vec<Finding>,
}

/// On-disk cache metadata.
#[derive(Debug, Serialize, Deserialize, Default)]
struct CacheMeta {
    env_hash: u64,
    files: HashMap<String, FileEntry>,
}

/// Unified project cache: parse results + findings + symbol summaries.
/// Facade over three cache layers (models, symbols, findings) plus the
/// meta bookkeeping they share. Method count ≥ 10 is structural, not a
/// smell — each layer exposes get/put atop the layered read/write helpers.
// cha:ignore large_class
pub struct ProjectCache {
    root: PathBuf,
    meta: CacheMeta,
    dirty: bool,
    /// L1 in-memory parse cache (avoids repeated disk reads within same process).
    mem_models: HashMap<u64, SourceModel>,
    /// L1 symbol-level summaries. Small enough to keep alongside models; lets
    /// `cached_symbols` skip the `SourceModel` deserialisation entirely.
    mem_symbols: HashMap<u64, SymbolIndex>,
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

fn cache_dir(root: &Path) -> PathBuf {
    root.join(".cha/cache")
}

fn content_hash(content: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut h);
    h.finish()
}

/// Layered L1-memory + L2-disk cache read. Templated over any
/// bincode-serialisable value; SourceModel and SymbolIndex share this.
fn get_layered<T: serde::de::DeserializeOwned + Clone>(
    mem: &mut HashMap<u64, T>,
    root: &Path,
    subdir: &str,
    chash: u64,
) -> Option<T> {
    if let Some(v) = mem.get(&chash) {
        return Some(v.clone());
    }
    let path = cache_dir(root)
        .join(subdir)
        .join(format!("{chash:016x}.bin"));
    let bytes = std::fs::read(&path).ok()?;
    let val: T = bincode::deserialize(&bytes).ok()?;
    mem.insert(chash, val.clone());
    Some(val)
}

/// Layered L1 + L2 write. Silently ignores serialisation or filesystem
/// errors — cache is a speed-up, not a correctness layer.
fn put_layered<T: serde::Serialize + Clone>(
    mem: &mut HashMap<u64, T>,
    root: &Path,
    subdir: &str,
    chash: u64,
    value: &T,
) {
    mem.insert(chash, value.clone());
    let dir = cache_dir(root).join(subdir);
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(bytes) = bincode::serialize(value) {
        let _ = std::fs::write(dir.join(format!("{chash:016x}.bin")), bytes);
    }
}

fn file_mtime_and_size(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((mtime, meta.len()))
}

impl ProjectCache {
    /// Open or create a cache for the given project root.
    pub fn open(project_root: &Path, env_hash: u64) -> Self {
        let dir = cache_dir(project_root);
        let meta_path = dir.join("meta.bin");
        let meta = std::fs::read(&meta_path)
            .ok()
            .and_then(|b| bincode::deserialize::<CacheMeta>(&b).ok())
            .unwrap_or_default();
        let meta = if meta.env_hash != env_hash {
            // Environment changed — full invalidation
            let _ = std::fs::remove_dir_all(&dir);
            CacheMeta {
                env_hash,
                ..Default::default()
            }
        } else {
            meta
        };
        Self {
            root: project_root.to_path_buf(),
            meta,
            dirty: false,
            mem_models: HashMap::new(),
            mem_symbols: HashMap::new(),
        }
    }

    /// Check if a file is unchanged (mtime + size match).
    /// Returns (is_unchanged, content_hash) — hash is 0 if unchanged and not yet computed.
    pub fn check_file(&self, rel_path: &str, path: &Path) -> FileStatus {
        let Some(entry) = self.meta.files.get(rel_path) else {
            return FileStatus::Changed;
        };
        if let Some((mtime, size)) = file_mtime_and_size(path)
            && mtime == entry.mtime_secs
            && size == entry.size
        {
            return FileStatus::Unchanged(entry.content_hash);
        }
        FileStatus::Changed
    }

    /// Get cached SourceModel: L1 memory → L2 disk.
    pub fn get_model(&mut self, chash: u64) -> Option<SourceModel> {
        get_layered(&mut self.mem_models, &self.root, "parse", chash)
    }

    /// Store a SourceModel in L1 + L2.
    pub fn put_model(&mut self, chash: u64, model: &SourceModel) {
        put_layered(&mut self.mem_models, &self.root, "parse", chash, model);
    }

    /// Get a cached `SymbolIndex`: L1 memory → L2 disk.
    /// Fast path for `cha deps`, LSP workspace-symbols and any other
    /// consumer that only needs structural information (names, relations,
    /// positions) without per-function-body analyse data.
    pub fn get_symbols(&mut self, chash: u64) -> Option<SymbolIndex> {
        get_layered(&mut self.mem_symbols, &self.root, "symbols", chash)
    }

    /// Store a `SymbolIndex` in L1 + L2. Called alongside `put_model` so
    /// both caches stay in lockstep.
    pub fn put_symbols(&mut self, chash: u64, idx: &SymbolIndex) {
        put_layered(&mut self.mem_symbols, &self.root, "symbols", chash, idx);
    }

    /// Get cached findings for a file.
    pub fn get_findings(&self, chash: u64) -> Option<Vec<Finding>> {
        let path = cache_dir(&self.root)
            .join("findings")
            .join(format!("{chash:016x}.bin"));
        let bytes = std::fs::read(&path).ok()?;
        let entry: FindingsEntry = bincode::deserialize(&bytes).ok()?;
        (entry.content_hash == chash).then_some(entry.findings)
    }

    /// Store findings for a file.
    pub fn put_findings(&mut self, chash: u64, findings: &[Finding]) {
        let dir = cache_dir(&self.root).join("findings");
        let _ = std::fs::create_dir_all(&dir);
        let entry = FindingsEntry {
            content_hash: chash,
            findings: findings.to_vec(),
        };
        if let Ok(bytes) = bincode::serialize(&entry) {
            let _ = std::fs::write(dir.join(format!("{chash:016x}.bin")), bytes);
        }
    }

    /// Update file metadata after processing.
    pub fn update_file_entry(
        &mut self,
        rel_path: String,
        path: &Path,
        chash: u64,
        imports: Vec<String>,
    ) {
        let (mtime_secs, size) = file_mtime_and_size(path).unwrap_or((0, 0));
        self.meta.files.insert(
            rel_path,
            FileEntry {
                mtime_secs,
                size,
                content_hash: chash,
                imports,
            },
        );
        self.dirty = true;
    }

    /// Get cached imports for a file (from meta, no disk I/O).
    pub fn get_imports(&self, rel_path: &str) -> Option<&[String]> {
        self.meta.files.get(rel_path).map(|e| e.imports.as_slice())
    }

    /// Flush metadata to disk and clean up orphan cache files.
    pub fn flush(&self) {
        if !self.dirty {
            return;
        }
        let dir = cache_dir(&self.root);
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(bytes) = bincode::serialize(&self.meta) {
            let _ = std::fs::write(dir.join("meta.bin"), bytes);
        }
        self.gc();
    }

    /// Remove orphan cache files not referenced by meta.
    fn gc(&self) {
        let hashes: std::collections::HashSet<String> = self
            .meta
            .files
            .values()
            .map(|e| format!("{:016x}.bin", e.content_hash))
            .collect();
        for subdir in &["parse", "findings", "symbols"] {
            let dir = cache_dir(&self.root).join(subdir);
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".bin") && !hashes.contains(&name) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        // Remove legacy analysis.json
        let legacy = cache_dir(&self.root).join("analysis.json");
        let _ = std::fs::remove_file(legacy);
    }
}

/// Result of checking a file against cache.
pub enum FileStatus {
    /// File unchanged — content hash from cache.
    Unchanged(u64),
    /// File changed or not in cache.
    Changed,
}

/// Compute a content hash.
pub fn hash_content(s: &str) -> u64 {
    content_hash(s)
}

/// Compute environment hash from config + plugins + cha binary fingerprint.
///
/// The binary fingerprint covers both cases that make cached SourceModels
/// stale:
/// - developer rebuilds cha after editing parser code,
/// - end user upgrades to a new cha release.
///
/// Both produce a different on-disk binary, so the binary's modification
/// time is sufficient. `CARGO_PKG_VERSION` was the old key, but it was
/// a strict subset of this: every release-version bump necessarily writes
/// a new binary (new mtime), and no parser change ever happens without a
/// rebuild (new mtime). Version-only tracking missed parser-behaviour
/// changes that shipped without a `cargo xtask bump` — this is what let
/// the header-declaration parser fix silently fail against stale caches.
pub fn env_hash(project_root: &Path, plugin_dirs: &[PathBuf]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    hash_cha_binary(&mut h);
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

/// Hash the cha binary's identity. Uses the running executable's mtime;
/// falls back to `CARGO_PKG_VERSION` if the executable path isn't
/// discoverable (unusual — sandboxed runners, embedded contexts). Either
/// path invalidates the cache on every new binary.
fn hash_cha_binary(h: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    match std::env::current_exe().and_then(|p| p.metadata()?.modified()) {
        Ok(mtime) => mtime.hash(h),
        Err(_) => env!("CARGO_PKG_VERSION").hash(h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceModel, TypeRef};
    use std::path::PathBuf;

    fn unique_tmp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        // Per-thread nanos can collide under parallel test execution —
        // fold in a monotonic counter so each call is unique.
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "cha-cache-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            SEQ.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn sample_model() -> SourceModel {
        SourceModel {
            language: "c".into(),
            total_lines: 10,
            functions: vec![],
            classes: vec![],
            imports: vec![],
            comments: vec![],
            type_aliases: vec![
                ("MyId".into(), "uint32_t".into()),
                ("Handle".into(), "void*".into()),
            ],
        }
    }

    /// Regression: `boundary_leak` used to parse files fresh because the
    /// cache appeared to drop typedef aliases on some C projects. After
    /// v1.11.0 tied the cache key to the binary's mtime, put/get should
    /// round-trip SourceModel faithfully — including `type_aliases`.
    #[test]
    fn cache_roundtrip_preserves_type_aliases() {
        let tmp = unique_tmp_dir();
        let mut cache = ProjectCache::open(&tmp, 0xdeadbeef);
        let model = sample_model();
        let chash: u64 = 0xdead_beef_1234_5678;
        // Register a file entry so flush()->gc() keeps the parse blob.
        cache.update_file_entry("x.c".into(), &tmp.join("nope"), chash, vec![]);
        cache.put_model(chash, &model);
        let got = cache.get_model(chash).expect("cached model present");
        assert_eq!(got.type_aliases, model.type_aliases);
        // Persist meta so reopening with the same env_hash doesn't
        // trigger the full-invalidation branch.
        cache.flush();
        drop(cache);
        let mut fresh = ProjectCache::open(&tmp, 0xdeadbeef);
        let from_disk = fresh.get_model(chash).expect("on-disk model present");
        assert_eq!(from_disk.type_aliases, model.type_aliases);
    }

    /// TypeRef origin information in parameter / return types also has to
    /// survive serialisation; boundary_leak's "interesting" check keys on
    /// `TypeOrigin::External`.
    #[test]
    fn cache_roundtrip_preserves_typeref_origin() {
        use crate::{FunctionInfo, TypeOrigin};
        let tmp = unique_tmp_dir();
        let mut cache = ProjectCache::open(&tmp, 0xdeadbeef);
        let model = SourceModel {
            language: "rust".into(),
            total_lines: 5,
            functions: vec![FunctionInfo {
                name: "f".into(),
                parameter_types: vec![TypeRef {
                    name: "ExtThing".into(),
                    raw: "ext::ExtThing".into(),
                    origin: TypeOrigin::External("ext".into()),
                }],
                ..Default::default()
            }],
            classes: vec![],
            imports: vec![],
            comments: vec![],
            type_aliases: vec![],
        };
        cache.put_model(99, &model);
        let got = cache.get_model(99).unwrap();
        let p = &got.functions[0].parameter_types[0];
        assert_eq!(p.name, "ExtThing");
        assert!(matches!(&p.origin, TypeOrigin::External(m) if m == "ext"));
    }

    /// `SymbolIndex` cache writes land in `symbols/{chash}.bin` independent
    /// of `parse/{chash}.bin`. Reopening the cache must recover them byte-
    /// for-byte — this is the invariant that lets `cached_symbols` skip
    /// `SourceModel` deserialisation entirely on warm runs.
    #[test]
    fn symbol_index_roundtrip_preserves_classes_and_functions() {
        use crate::{ClassSymbol, FunctionSymbol, SymbolIndex};
        let tmp = unique_tmp_dir();
        let mut cache = ProjectCache::open(&tmp, 0xdeadbeef);
        let idx = SymbolIndex {
            language: "c".into(),
            total_lines: 42,
            imports: vec![],
            classes: vec![ClassSymbol {
                name: "Foo".into(),
                parent_name: Some("Base".into()),
                is_interface: false,
                is_exported: true,
                method_count: 3,
                has_behavior: true,
                field_names: vec!["x".into()],
                field_types: vec!["int".into()],
                start_line: 10,
                end_line: 20,
                ..Default::default()
            }],
            functions: vec![FunctionSymbol {
                name: "bar".into(),
                is_exported: true,
                parameter_count: 2,
                called_functions: vec!["helper".into(), "log".into()],
                start_line: 30,
                end_line: 40,
                ..Default::default()
            }],
            type_aliases: vec![("Handle".into(), "void*".into())],
        };
        let chash = 0x1234_abcd_u64;
        cache.update_file_entry("t.c".into(), &tmp.join("nope"), chash, vec![]);
        cache.put_symbols(chash, &idx);
        let got_l1 = cache.get_symbols(chash).expect("L1 hit");
        assert_eq!(got_l1.classes[0].name, "Foo");
        assert_eq!(got_l1.functions[0].called_functions.len(), 2);
        assert_eq!(got_l1.type_aliases[0].0, "Handle");
        // Force L2 round-trip.
        cache.flush();
        drop(cache);
        let mut fresh = ProjectCache::open(&tmp, 0xdeadbeef);
        let from_disk = fresh.get_symbols(chash).expect("L2 hit");
        assert_eq!(from_disk.classes[0].parent_name.as_deref(), Some("Base"));
        assert_eq!(from_disk.functions[0].parameter_count, 2);
    }
}
