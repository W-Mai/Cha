//! Cache-aware parsing entry points. Two flavours:
//!
//! - [`cached_parse`] — returns full `SourceModel`, used by `analyze`
//!   and anything that needs per-function complexity, TypeRef origins,
//!   or body hashes.
//! - [`cached_symbols`] — returns `SymbolIndex`, used by `cha deps`
//!   (and future LSP workspace-symbols / summary commands). Skips
//!   `SourceModel` deserialisation on warm cache hits, so lightweight
//!   callers don't pay for body-level data they ignore.
//!
//! Both share `ProjectCache`'s two-tier storage. `cached_parse` writes
//! both `parse/{chash}.bin` (model) and `symbols/{chash}.bin` (symbol
//! index) so the next `cached_symbols` call hits the fast path.

use std::path::Path;

/// Parse a file with cache support: mtime check → content hash → parse on miss.
/// Populates both the model and the symbol-index cache.
pub(crate) fn cached_parse(
    path: &Path,
    cache: &mut cha_core::ProjectCache,
    cwd: &Path,
) -> Option<(String, cha_core::SourceModel)> {
    let rel = path
        .strip_prefix(cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    // Fast path: mtime+size unchanged → use cached model without reading file
    if let cha_core::FileStatus::Unchanged(chash) = cache.check_file(&rel, path)
        && let Some(model) = cache.get_model(chash)
    {
        return Some((rel, model));
    }
    // Slow path: read file, hash, check/parse
    let content = std::fs::read_to_string(path).ok()?;
    let chash = cha_core::hash_content(&content);
    if let Some(model) = cache.get_model(chash) {
        let imports = model.imports.iter().map(|i| i.source.clone()).collect();
        cache.update_file_entry(rel.clone(), path, chash, imports);
        // Reuse the symbol index if present; otherwise derive and store.
        if cache.get_symbols(chash).is_none() {
            let idx = cha_core::SymbolIndex::from_source_model(&model);
            cache.put_symbols(chash, &idx);
        }
        return Some((rel, model));
    }
    let file = cha_core::SourceFile::new(path.to_path_buf(), content);
    let model = cha_parser::parse_file(&file)?;
    cache.put_model(chash, &model);
    cache.put_symbols(chash, &cha_core::SymbolIndex::from_source_model(&model));
    let imports = model.imports.iter().map(|i| i.source.clone()).collect();
    cache.update_file_entry(rel.clone(), path, chash, imports);
    Some((rel, model))
}

/// Symbol-level fast path. Skips `SourceModel` deserialisation on warm
/// cache hits — `symbols/{chash}.bin` is an order of magnitude smaller.
/// Falls back to `cached_parse` when the symbol cache is missing
/// (e.g. first run, or after a parser-code change that bumped env_hash
/// and wiped both caches).
pub(crate) fn cached_symbols(
    path: &Path,
    cache: &mut cha_core::ProjectCache,
    cwd: &Path,
) -> Option<(String, cha_core::SymbolIndex)> {
    let rel = path
        .strip_prefix(cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    if let cha_core::FileStatus::Unchanged(chash) = cache.check_file(&rel, path)
        && let Some(idx) = cache.get_symbols(chash)
    {
        return Some((rel, idx));
    }
    // Miss: fall through to the full parse path, which populates both
    // caches. Derive the symbol index from the fresh model directly
    // instead of round-tripping through `get_symbols`.
    let (rel, model) = cached_parse(path, cache, cwd)?;
    Some((rel, cha_core::SymbolIndex::from_source_model(&model)))
}
