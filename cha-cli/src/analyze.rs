use std::path::{Path, PathBuf};

use cha_core::{
    AnalysisContext, Config, Finding, JsonReporter, LlmContextReporter, PluginRegistry, Reporter,
    SarifReporter, Severity, SourceFile, TerminalReporter,
};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::{DiffMode, FailLevel, Format, collect_files, diff, git_diff_files};

pub(crate) fn cmd_analyze(
    paths: &[String],
    format: &Format,
    fail_on: Option<&FailLevel>,
    diff_mode: DiffMode,
    plugin_filter: &[String],
    baseline_path: Option<&str>,
    output_path: Option<&str>,
) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = Config::load(&cwd);
    let (files, diff_map) = resolve_diff_files(paths, &diff_mode, &cwd);
    let files = filter_excluded(files, &root_config.exclude, &cwd);

    if files.is_empty() {
        println!("No files to analyze.");
        return 0;
    }

    let all_findings = run_analysis(&files, &cwd, plugin_filter);
    let all_findings = match diff_map {
        Some(ref dm) => diff::filter_by_diff(all_findings, dm),
        None => all_findings,
    };
    let all_findings = if let Some(bp) = baseline_path {
        match cha_core::Baseline::load(Path::new(bp)) {
            Some(bl) => bl.filter_new(all_findings, &cwd),
            None => {
                eprintln!("warning: baseline file not found: {bp}");
                all_findings
            }
        }
    } else {
        all_findings
    };
    if matches!(format, Format::Html) {
        print_html_report(
            &all_findings,
            &files,
            output_path,
            &root_config.debt_weights,
        );
    } else {
        print_report(&all_findings, format, &files, &root_config.debt_weights);
    }
    exit_code(&all_findings, fail_on)
}

/// Resolve file list and optional diff map based on diff mode.
fn resolve_diff_files(
    paths: &[String],
    mode: &DiffMode,
    cwd: &Path,
) -> (Vec<PathBuf>, Option<diff::DiffMap>) {
    match mode {
        DiffMode::Stdin => {
            let dm = diff::parse_unified_diff(&read_stdin());
            let files = dm.keys().map(|p| cwd.join(p)).collect();
            (files, Some(dm))
        }
        DiffMode::Git => {
            let dm = diff::git_diff_ranges();
            let files = if dm.is_empty() {
                collect_files(paths)
            } else {
                dm.keys().map(|p| cwd.join(p)).collect()
            };
            (files, Some(dm))
        }
        DiffMode::None => (resolve_files(paths, false), None),
    }
}

/// Filter out files matching exclude glob patterns from config.
pub(crate) fn filter_excluded(
    files: Vec<PathBuf>,
    patterns: &[String],
    root: &Path,
) -> Vec<PathBuf> {
    if patterns.is_empty() {
        return files;
    }
    let matchers: Vec<glob::Pattern> = patterns
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();
    files
        .into_iter()
        .filter(|f| {
            let rel = f.strip_prefix(root).unwrap_or(f);
            let s = rel.to_string_lossy();
            !matchers.iter().any(|m| m.matches(&s))
        })
        .collect()
}

/// Read all of stdin into a string.
fn read_stdin() -> String {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .unwrap_or_default();
    buf
}

pub(crate) fn resolve_files(paths: &[String], diff: bool) -> Vec<PathBuf> {
    if diff {
        let diff_files = git_diff_files();
        if diff_files.is_empty() {
            collect_files(paths)
        } else {
            diff_files
        }
    } else {
        collect_files(paths)
    }
}

/// Analyze files in parallel using rayon, with per-file config inheritance.
pub(crate) fn run_analysis(
    files: &[PathBuf],
    project_root: &Path,
    plugin_filter: &[String],
) -> Vec<Finding> {
    let cache = open_cache(project_root, !plugin_filter.is_empty());

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );
    let results: Vec<Finding> = files
        .par_iter()
        .flat_map(|path| {
            let findings = analyze_one(path, project_root, plugin_filter, &cache);
            pb.inc(1);
            findings
        })
        .collect();
    pb.finish_and_clear();

    if let Some(cache) = cache
        && let Ok(c) = cache.into_inner()
    {
        c.flush();
    }
    results
}

fn open_cache(
    project_root: &Path,
    no_cache: bool,
) -> Option<std::sync::Mutex<cha_core::AnalysisCache>> {
    use cha_core::AnalysisCache;
    if no_cache {
        return None;
    }
    let plugin_dirs = vec![
        project_root.join(".cha/plugins"),
        dirs::home_dir().unwrap_or_default().join(".cha/plugins"),
    ];
    let env_hash = AnalysisCache::env_hash(project_root, &plugin_dirs);
    Some(std::sync::Mutex::new(AnalysisCache::open(
        project_root,
        env_hash,
    )))
}

fn analyze_one(
    path: &Path,
    project_root: &Path,
    plugin_filter: &[String],
    cache: &Option<std::sync::Mutex<cha_core::AnalysisCache>>,
) -> Vec<Finding> {
    use cha_core::AnalysisCache;

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let rel = path
        .strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let content_hash = AnalysisCache::hash_content(&content);

    if let Some(cache) = cache
        && let Ok(c) = cache.lock()
        && let Some(cached) = c.get(&rel, content_hash)
    {
        return cached.to_vec();
    }

    let findings = analyze_file_with_content(path, &content, project_root, plugin_filter);

    if let Some(cache) = cache
        && let Ok(mut c) = cache.lock()
    {
        c.put(rel, content_hash, findings.clone());
    }
    findings
}

fn analyze_file_with_content(
    path: &Path,
    content: &str,
    project_root: &Path,
    plugin_filter: &[String],
) -> Vec<Finding> {
    let file = SourceFile::new(path.to_path_buf(), content.to_string());
    let model = match cha_parser::parse_file(&file) {
        Some(m) => m,
        None => return vec![],
    };
    let config = Config::load_for_file(path, project_root);
    let registry = PluginRegistry::from_config(&config, project_root);
    let ctx = AnalysisContext {
        file: &file,
        model: &model,
    };
    registry
        .plugins()
        .par_iter()
        .filter(|p| plugin_filter.is_empty() || plugin_filter.iter().any(|f| f == p.name()))
        .flat_map(|p| p.analyze(&ctx))
        .collect()
}

fn print_html_report(
    findings: &[Finding],
    files: &[PathBuf],
    output_path: Option<&str>,
    weights: &cha_core::DebtWeights,
) {
    let file_data: Vec<(String, usize)> = files
        .iter()
        .filter_map(|p| {
            let c = std::fs::read_to_string(p).ok()?;
            Some((p.to_string_lossy().to_string(), c.lines().count()))
        })
        .collect();
    let mut scores = cha_core::score_files(findings, &file_data, weights);
    scores.sort_by(|a, b| {
        b.grade
            .cmp(&a.grade)
            .then(b.debt_minutes.cmp(&a.debt_minutes))
    });
    let file_contents: Vec<(String, String)> = files
        .iter()
        .filter_map(|p| {
            let c = std::fs::read_to_string(p).ok()?;
            Some((p.to_string_lossy().to_string(), c))
        })
        .collect();
    let html = cha_core::html_reporter::render_html(findings, &scores, &file_contents);
    match output_path {
        Some(path) => {
            std::fs::write(path, &html).unwrap_or_else(|e| eprintln!("Failed to write: {e}"));
            println!("Report written to {path}");
        }
        None => println!("{html}"),
    }
}

fn compute_file_lines(files: &[PathBuf]) -> Vec<(String, usize)> {
    files
        .iter()
        .filter_map(|p| {
            let c = std::fs::read_to_string(p).ok()?;
            Some((p.to_string_lossy().to_string(), c.lines().count()))
        })
        .collect()
}

fn print_report(
    findings: &[Finding],
    format: &Format,
    files: &[PathBuf],
    weights: &cha_core::DebtWeights,
) {
    match format {
        Format::Json | Format::Sarif => {
            let scores = cha_core::score_files(findings, &compute_file_lines(files), weights);
            let output = if matches!(format, Format::Json) {
                JsonReporter.render_with_scores(findings, &scores)
            } else {
                SarifReporter.render_with_scores(findings, &scores)
            };
            println!("{output}");
        }
        Format::Terminal => {
            println!("{}", TerminalReporter.render(findings));
            if !findings.is_empty() {
                print_health_scores(findings, files, weights);
            }
        }
        Format::Llm => println!("{}", LlmContextReporter.render(findings)),
        Format::Html => unreachable!("HTML handled separately"),
    }
}

fn print_health_scores(findings: &[Finding], files: &[PathBuf], weights: &cha_core::DebtWeights) {
    let mut scores = cha_core::score_files(findings, &compute_file_lines(files), weights);
    scores.sort_by(|a, b| {
        b.grade
            .cmp(&a.grade)
            .then(b.debt_minutes.cmp(&a.debt_minutes))
    });
    let worst: Vec<_> = scores
        .iter()
        .filter(|s| s.grade > cha_core::Grade::A)
        .collect();
    if !worst.is_empty() {
        println!("\nHealth scores:");
        for s in &worst {
            println!("  {} {} (~{}min debt)", s.grade, s.path, s.debt_minutes);
        }
    }
    let total: u32 = scores.iter().map(|s| s.debt_minutes).sum();
    if total > 0 {
        let grade_count = |g: cha_core::Grade| scores.iter().filter(|s| s.grade == g).count();
        println!(
            "\nTech debt: ~{} | A:{} B:{} C:{} D:{} F:{}",
            format_duration(total),
            grade_count(cha_core::Grade::A),
            grade_count(cha_core::Grade::B),
            grade_count(cha_core::Grade::C),
            grade_count(cha_core::Grade::D),
            grade_count(cha_core::Grade::F),
        );
    }
}

fn format_duration(minutes: u32) -> String {
    if minutes < 60 {
        format!("{minutes}min")
    } else {
        let h = minutes / 60;
        let m = minutes % 60;
        if m == 0 {
            format!("{h}h")
        } else {
            format!("{h}h {m}min")
        }
    }
}

pub(crate) fn exit_code(findings: &[Finding], fail_on: Option<&FailLevel>) -> i32 {
    if let Some(level) = fail_on {
        let threshold = match level {
            FailLevel::Hint => Severity::Hint,
            FailLevel::Warning => Severity::Warning,
            FailLevel::Error => Severity::Error,
        };
        if findings.iter().any(|f| f.severity >= threshold) {
            return 1;
        }
    }
    0
}
