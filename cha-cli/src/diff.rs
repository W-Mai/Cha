use std::collections::HashMap;
use std::path::PathBuf;

/// A range of changed lines (1-indexed, inclusive).
pub type LineRange = (usize, usize);

/// Map from file path to its changed line ranges.
pub type DiffMap = HashMap<PathBuf, Vec<LineRange>>;

/// Parse unified diff output into a DiffMap.
/// Handles `--- a/path` / `+++ b/path` and `@@ -old +new,count @@` hunks.
pub fn parse_unified_diff(input: &str) -> DiffMap {
    let mut map = DiffMap::new();
    let mut current_file: Option<PathBuf> = None;

    for line in input.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_file = Some(PathBuf::from(path));
        } else if line.starts_with("+++ /dev/null") {
            current_file = None; // deleted file
        } else if line.starts_with("@@ ")
            && let Some(ref file) = current_file
            && let Some(range) = parse_hunk_header(line)
        {
            map.entry(file.clone()).or_default().push(range);
        }
    }
    map
}

/// Parse `@@ -old,count +new,count @@` into (start, end) for the new side.
fn parse_hunk_header(line: &str) -> Option<LineRange> {
    // Find the +N,M or +N part
    let plus_part = line.split_whitespace().find(|s| s.starts_with('+'))?;
    let nums = plus_part.trim_start_matches('+');
    let (start, count) = if let Some((s, c)) = nums.split_once(',') {
        (s.parse::<usize>().ok()?, c.parse::<usize>().ok()?)
    } else {
        (nums.parse::<usize>().ok()?, 1)
    };
    if count == 0 {
        return None; // pure deletion, no new lines
    }
    Some((start, start + count - 1))
}

/// Get diff map from `git diff -U0 HEAD`.
pub fn git_diff_ranges() -> DiffMap {
    let output = std::process::Command::new("git")
        .args(["diff", "-U0", "HEAD"])
        .output();
    match output {
        Ok(o) if o.status.success() => parse_unified_diff(&String::from_utf8_lossy(&o.stdout)),
        _ => DiffMap::new(),
    }
}

/// Filter findings to only those overlapping with changed line ranges.
pub fn filter_by_diff(
    findings: Vec<cha_core::Finding>,
    diff_map: &DiffMap,
) -> Vec<cha_core::Finding> {
    if diff_map.is_empty() {
        return findings;
    }
    findings
        .into_iter()
        .filter(|f| {
            let Some(ranges) = diff_map.get(&f.location.path) else {
                return false;
            };
            ranges
                .iter()
                .any(|&(ds, de)| f.location.start_line <= de && f.location.end_line >= ds)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hunk_single_line() {
        let diff = "+++ b/src/main.rs\n@@ -10,0 +11 @@ fn foo\n";
        let map = parse_unified_diff(diff);
        assert_eq!(map[&PathBuf::from("src/main.rs")], vec![(11, 11)]);
    }

    #[test]
    fn parse_hunk_multi_line() {
        let diff = "+++ b/lib.rs\n@@ -5,3 +5,10 @@ fn bar\n";
        let map = parse_unified_diff(diff);
        assert_eq!(map[&PathBuf::from("lib.rs")], vec![(5, 14)]);
    }

    #[test]
    fn parse_multiple_hunks() {
        let diff = "+++ b/a.rs\n@@ -1,0 +1,3 @@\n@@ -10,0 +14,2 @@\n";
        let map = parse_unified_diff(diff);
        assert_eq!(map[&PathBuf::from("a.rs")], vec![(1, 3), (14, 15)]);
    }

    #[test]
    fn parse_deletion_only_skipped() {
        let diff = "+++ b/a.rs\n@@ -5,3 +5,0 @@\n";
        let map = parse_unified_diff(diff);
        let ranges = map.get(&PathBuf::from("a.rs"));
        assert!(ranges.is_none() || ranges.unwrap().is_empty());
    }

    #[test]
    fn parse_deleted_file_skipped() {
        let diff = "+++ /dev/null\n@@ -1,10 +0,0 @@\n";
        let map = parse_unified_diff(diff);
        assert!(map.is_empty());
    }

    #[test]
    fn filter_keeps_overlapping() {
        let mut map = DiffMap::new();
        map.insert(PathBuf::from("a.rs"), vec![(10, 20)]);

        let findings = vec![
            make_finding("a.rs", 5, 8),   // before range
            make_finding("a.rs", 15, 18), // inside range
            make_finding("a.rs", 8, 12),  // overlaps start
            make_finding("a.rs", 25, 30), // after range
            make_finding("b.rs", 15, 18), // different file
        ];
        let filtered = filter_by_diff(findings, &map);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].location.start_line, 15);
        assert_eq!(filtered[1].location.start_line, 8);
    }

    fn make_finding(path: &str, start: usize, end: usize) -> cha_core::Finding {
        cha_core::Finding {
            smell_name: "test".into(),
            category: cha_core::SmellCategory::Bloaters,
            severity: cha_core::Severity::Warning,
            location: cha_core::Location {
                path: PathBuf::from(path),
                start_line: start,
                end_line: end,
                name: None,
            },
            message: "test".into(),
            suggested_refactorings: vec![],
            ..Default::default()
        }
    }
}
