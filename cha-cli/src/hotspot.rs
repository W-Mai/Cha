use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use cha_core::SourceFile;

use crate::Format;

/// Hotspot = git change frequency × code complexity.
///
/// ## References
///
/// [1] A. Tornhill, "Your Code as a Crime Scene," Pragmatic Bookshelf, 2015.
///     doi: 10.1007/978-1-4842-1628-2.

#[derive(serde::Serialize)]
struct Hotspot {
    path: String,
    commits: usize,
    complexity: usize,
    score: f64,
}

pub fn cmd_hotspot(count: usize, top: usize, format: &Format) {
    let freq = git_change_frequency(count);
    if freq.is_empty() {
        println!("No git history found.");
        return;
    }

    let mut hotspots: Vec<Hotspot> = freq
        .into_iter()
        .filter_map(|(path, commits)| {
            let complexity = file_complexity(&path)?;
            let score = commits as f64 * complexity as f64;
            Some(Hotspot {
                path,
                commits,
                complexity,
                score,
            })
        })
        .collect();

    hotspots.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hotspots.truncate(top);

    match format {
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&hotspots).unwrap_or_default()
        ),
        _ => print_terminal(&hotspots),
    }
}

fn git_change_frequency(n: usize) -> HashMap<String, usize> {
    let output = Command::new("git")
        .args(["log", "--format=", "--name-only", "-n", &n.to_string()])
        .output()
        .ok();
    let Some(output) = output else {
        return HashMap::new();
    };
    let mut freq = HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        *freq.entry(line.to_string()).or_default() += 1;
    }
    freq
}

fn file_complexity(path: &str) -> Option<usize> {
    let content = std::fs::read_to_string(path).ok()?;
    let file = SourceFile::new(PathBuf::from(path), content);
    let model = cha_parser::parse_file(&file)?;
    let total: usize = model.functions.iter().map(|f| f.complexity).sum();
    Some(total.max(1))
}

fn print_terminal(hotspots: &[Hotspot]) {
    if hotspots.is_empty() {
        return;
    }
    let max_score = hotspots.first().map(|h| h.score).unwrap_or(1.0).max(1.0);
    let width = 30;

    println!("Hotspots (commits × complexity):\n");
    for h in hotspots {
        let bar_len = (h.score / max_score * width as f64) as usize;
        let bar: String = "█".repeat(bar_len);
        println!(
            "  {:>5.0} │{} {} ({}c × {}cx)",
            h.score, bar, h.path, h.commits, h.complexity
        );
    }
    println!();
}
