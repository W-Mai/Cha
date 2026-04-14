use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Format;
use crate::analyze;

/// Entry for one commit in the trend.
#[derive(serde::Serialize)]
struct TrendPoint {
    hash: String,
    short_hash: String,
    message: String,
    findings: usize,
    errors: usize,
    warnings: usize,
    hints: usize,
}

pub fn cmd_trend(count: usize, format: &Format) {
    let commits = recent_commits(count);
    if commits.is_empty() {
        println!("No commits found.");
        return;
    }

    let points: Vec<TrendPoint> = commits
        .iter()
        .map(|(hash, msg)| analyze_commit(hash, msg))
        .collect();

    match format {
        Format::Json => print_json(&points),
        _ => print_ascii(&points),
    }
}

fn recent_commits(n: usize) -> Vec<(String, String)> {
    let output = Command::new("git")
        .args(["log", "--format=%H %s", "-n", &n.to_string()])
        .output()
        .ok();
    let Some(output) = output else { return vec![] };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (hash, msg) = line.split_once(' ')?;
            Some((hash.to_string(), msg.to_string()))
        })
        .collect()
}

fn analyze_commit(hash: &str, message: &str) -> TrendPoint {
    let short = &hash[..7];
    let tmp = std::env::temp_dir().join(format!("cha-trend-{short}"));

    // Create worktree
    let ok = Command::new("git")
        .args(["worktree", "add", "--detach", tmp.to_str().unwrap(), hash])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());

    let point = if ok {
        analyze_dir(&tmp, hash, message)
    } else {
        TrendPoint {
            hash: hash.to_string(),
            short_hash: short.to_string(),
            message: message.to_string(),
            findings: 0,
            errors: 0,
            warnings: 0,
            hints: 0,
        }
    };

    // Cleanup
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force", tmp.to_str().unwrap()])
        .stderr(std::process::Stdio::null())
        .status();

    point
}

fn analyze_dir(dir: &Path, hash: &str, message: &str) -> TrendPoint {
    let short = &hash[..7];

    let files = collect_source_files(dir);
    let findings = analyze::run_analysis(&files, dir, &[]);

    eprintln!("  {short} — {} issues", findings.len());

    let errors = findings
        .iter()
        .filter(|f| f.severity == cha_core::Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|f| f.severity == cha_core::Severity::Warning)
        .count();
    let hints = findings.len() - errors - warnings;

    TrendPoint {
        hash: hash.to_string(),
        short_hash: short.to_string(),
        message: message.to_string(),
        findings: findings.len(),
        errors,
        warnings,
        hints,
    }
}

fn collect_source_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(dir, &mut files);
    files
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.')
            || matches!(
                name_str.as_ref(),
                "target" | "node_modules" | "dist" | "build"
            )
        {
            continue;
        }
        if path.is_dir() {
            walk(&path, files);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext,
                    "rs" | "ts" | "tsx" | "py" | "go" | "c" | "h" | "cpp" | "cc" | "cxx"
                )
            })
        {
            files.push(path);
        }
    }
}

fn print_json(points: &[TrendPoint]) {
    println!(
        "{}",
        serde_json::to_string_pretty(points).unwrap_or_default()
    );
}

fn print_ascii(points: &[TrendPoint]) {
    if points.is_empty() {
        return;
    }
    let max = points.iter().map(|p| p.findings).max().unwrap_or(1).max(1);
    let width = 40;

    println!("Trend ({} commits, newest first):\n", points.len());
    for p in points {
        let bar_len = (p.findings as f64 / max as f64 * width as f64) as usize;
        let bar: String = "█".repeat(bar_len);
        println!(
            "  {} {:>4} │{} {}",
            p.short_hash, p.findings, bar, p.message
        );
    }
    println!();
}
