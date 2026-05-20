//! Path-shape utility helpers shared across detectors.
//!
//! These functions used to live as private duplicated impls inside
//! `boundary_leak` and `module_envy`. Centralized here so `ProjectQuery`
//! implementations and standalone helpers stay aligned.

use std::path::Path;

/// True if `path` is under a test directory or named like a test file.
///
/// Recognized patterns:
/// - segment match: `tests/`, `test/`, `__tests__/`, `__mocks__/`,
///   `spec/`, `specs/`
/// - filename: starts with `test_` / ends with `_test`, `.test`,
///   `.spec`, `_spec`
pub fn is_test_path(path: &Path) -> bool {
    const TEST_DIRS: &[&str] = &["tests", "test", "__tests__", "__mocks__", "spec", "specs"];
    let has_test_segment = path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .is_some_and(|s| TEST_DIRS.contains(&s))
    });
    if has_test_segment {
        return true;
    }
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        && (stem.starts_with("test_")
            || stem.ends_with("_test")
            || stem.ends_with(".test")
            || stem.ends_with(".spec")
            || stem.ends_with("_spec"))
    {
        return true;
    }
    false
}
