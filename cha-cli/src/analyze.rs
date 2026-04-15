use std::path::{Path, PathBuf};

use cha_core::{
    AnalysisContext, Config, Finding, JsonReporter, LlmContextReporter, PluginRegistry, Reporter,
    SarifReporter, Severity, SourceFile, TerminalReporter,
};
use rayon::prelude::*;

use crate::{DiffMode, FailLevel, Format, collect_files, diff, git_diff_files};

/// CLI --strictness override, applied to every file's config.
static STRICTNESS_OVERRIDE: std::sync::OnceLock<cha_core::Strictness> = std::sync::OnceLock::new();

pub(crate) struct AnalyzeOpts<'a> {
    pub paths: &'a [String],
    pub format: &'a Format,
    pub fail_on: Option<&'a FailLevel>,
    pub diff_mode: DiffMode,
    pub plugin_filter: &'a [String],
    pub baseline_path: Option<&'a str>,
    pub output_path: Option<&'a str>,
    pub strictness: Option<&'a str>,
}

pub(crate) fn cmd_analyze(opts: &AnalyzeOpts) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = Config::load(&cwd);
    if let Some(s) = opts.strictness.and_then(cha_core::Strictness::parse) {
        let _ = STRICTNESS_OVERRIDE.set(s);
    }
    let (files, diff_map) = resolve_diff_files(opts.paths, &opts.diff_mode, &cwd);
    let files = filter_excluded(files, &root_config.exclude, &cwd);

    if files.is_empty() {
        println!("No files to analyze.");
        return 0;
    }

    let mut all_findings = run_analysis(&files, &cwd, opts.plugin_filter);
    all_findings.extend(run_post_analysis(&files, &cwd, opts.plugin_filter));
    let all_findings = apply_filters(all_findings, &diff_map, opts.baseline_path, &cwd);

    if matches!(opts.format, Format::Html) {
        print_html_report(
            &all_findings,
            &files,
            opts.output_path,
            &root_config.debt_weights,
        );
    } else {
        print_report(
            &all_findings,
            opts.format,
            &files,
            &root_config.debt_weights,
        );
    }
    exit_code(&all_findings, opts.fail_on)
}

/// Post-analysis passes: name and description (cross-file, not per-file Plugin trait).
pub(crate) const POST_ANALYSIS_PASSES: &[(&str, &str)] = &[
    (
        "unstable_dependency",
        "Depends on less stable modules (Martin's instability)",
    ),
    ("test_ratio", "Low test-to-production code ratio"),
    ("tangled_change", "Commit touches too many directories"),
    (
        "knowledge_distribution",
        "File has only one contributor (bus factor)",
    ),
];

fn run_post_analysis(files: &[PathBuf], cwd: &Path, plugin_filter: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let pass = |name: &str| plugin_filter.is_empty() || plugin_filter.iter().any(|f| f == name);
    if pass("unstable_dependency") {
        findings.extend(detect_unstable_deps(files, cwd));
    }
    if pass("test_ratio") {
        findings.extend(check_test_ratio(files));
    }
    if pass("tangled_change") {
        findings.extend(crate::tangled::detect_tangled(50, 4));
    }
    if pass("knowledge_distribution") {
        findings.extend(detect_bus_factor(files, cwd));
    }
    findings
}

fn apply_filters(
    findings: Vec<Finding>,
    diff_map: &Option<diff::DiffMap>,
    baseline_path: Option<&str>,
    cwd: &Path,
) -> Vec<Finding> {
    let findings = match diff_map {
        Some(dm) => diff::filter_by_diff(findings, dm),
        None => findings,
    };
    if let Some(bp) = baseline_path {
        match cha_core::Baseline::load(Path::new(bp)) {
            Some(bl) => bl.filter_new(findings, cwd),
            None => {
                eprintln!("warning: baseline file not found: {bp}");
                findings
            }
        }
    } else {
        findings
    }
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

    let pb = crate::new_progress_bar(files.len() as u64);
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
    let mut config =
        Config::load_for_file(path, project_root).resolve_for_language(&model.language);
    if let Some(s) = STRICTNESS_OVERRIDE.get() {
        config.set_strictness(s.clone());
    }
    let registry = PluginRegistry::from_config(&config, project_root);
    let ctx = AnalysisContext {
        file: &file,
        model: &model,
    };
    let findings: Vec<Finding> = registry
        .plugins()
        .par_iter()
        .filter(|p| plugin_filter.is_empty() || plugin_filter.iter().any(|f| f == p.name()))
        .flat_map(|p| p.analyze(&ctx))
        .collect();
    cha_core::filter_ignored(findings, content)
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

/// Detect Unstable Dependencies using Martin's instability metric.
///
/// ## References
///
/// [1] R. C. Martin, "Agile Software Development," Prentice Hall, 2003. Ch. 20.
/// [2] F. Arcelli Fontana et al., ECSA 2019. doi: 10.1145/3344948.3344982.
fn detect_unstable_deps(files: &[PathBuf], cwd: &Path) -> Vec<Finding> {
    let file_imports = build_file_imports(files, cwd);
    let known: std::collections::HashSet<&str> = file_imports.keys().map(|s| s.as_str()).collect();
    let ca = compute_afferent(&file_imports, &known);

    let instability = |file: &str| -> f64 {
        let ce = file_imports.get(file).map(|v| v.len()).unwrap_or(0) as f64;
        let ca_val = ca.get(file).copied().unwrap_or(0) as f64;
        if ce + ca_val == 0.0 {
            0.0
        } else {
            ce / (ca_val + ce)
        }
    };

    file_imports
        .iter()
        .filter_map(|(file, imports)| {
            let my_i = instability(file);
            let (target, ti) = imports.iter().find_map(|imp| {
                let t = known
                    .iter()
                    .find(|&&k| imp.contains(k) || k.contains(imp.as_str()))?;
                let ti = instability(t);
                (my_i < ti && (ti - my_i) > 0.2).then_some((*t, ti))
            })?;
            Some(Finding {
                smell_name: "unstable_dependency".into(),
                category: cha_core::SmellCategory::Couplers,
                severity: cha_core::Severity::Hint,
                location: cha_core::Location {
                    path: PathBuf::from(file),
                    start_line: 1,
                    end_line: 1,
                    name: None,
                },
                message: format!(
                    "`{file}` (I={my_i:.2}) depends on `{target}` (I={ti:.2}) which is less stable"
                ),
                suggested_refactorings: vec![
                    "Depend on abstractions".into(),
                    "Stable Dependencies Principle".into(),
                ],
            })
        })
        .collect()
}

fn build_file_imports(
    files: &[PathBuf],
    cwd: &Path,
) -> std::collections::HashMap<String, Vec<String>> {
    let mut map = std::collections::HashMap::new();
    for path in files {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let file = SourceFile::new(path.clone(), content);
        let Some(model) = cha_parser::parse_file(&file) else {
            continue;
        };
        let rel = path
            .strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        map.insert(
            rel,
            model.imports.iter().map(|i| i.source.clone()).collect(),
        );
    }
    map
}

fn compute_afferent<'a>(
    file_imports: &'a std::collections::HashMap<String, Vec<String>>,
    known: &std::collections::HashSet<&'a str>,
) -> std::collections::HashMap<&'a str, usize> {
    let mut ca = std::collections::HashMap::new();
    for imports in file_imports.values() {
        for imp in imports {
            if let Some(&k) = known
                .iter()
                .find(|&&k| imp.contains(k) || k.contains(imp.as_str()))
            {
                *ca.entry(k).or_default() += 1;
            }
        }
    }
    ca
}

/// Detect files with bus factor = 1 (only one author).
///
/// ## References
///
/// [1] N. Nagappan et al., "The influence of organizational structure on
///     software quality," ICSE 2008. doi: 10.1145/1368088.1368122.
fn detect_bus_factor(files: &[PathBuf], cwd: &Path) -> Vec<Finding> {
    let file_authors = git_file_authors();

    files
        .iter()
        .filter_map(|path| {
            let rel = path.strip_prefix(cwd).unwrap_or(path);
            let authors = file_authors.get(rel.to_str()?)?;
            (authors.len() == 1 && path.metadata().map(|m| m.len() > 500).unwrap_or(false)).then(
                || Finding {
                    smell_name: "bus_factor".into(),
                    category: cha_core::SmellCategory::ChangePreventers,
                    severity: cha_core::Severity::Hint,
                    location: cha_core::Location {
                        path: rel.to_path_buf(),
                        start_line: 1,
                        end_line: 1,
                        name: None,
                    },
                    message: format!(
                        "`{}` has only 1 contributor — bus factor risk",
                        rel.display()
                    ),
                    suggested_refactorings: vec!["Pair programming".into(), "Code review".into()],
                },
            )
        })
        .collect()
}

/// Single `git log` call to build file → authors map.
fn git_file_authors() -> std::collections::HashMap<String, std::collections::HashSet<String>> {
    let output = std::process::Command::new("git")
        .args(["log", "--format=%aN", "-n", "200", "--name-only"])
        .output()
        .ok();
    let Some(output) = output else {
        return Default::default();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut file_authors: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    let mut current_author = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.contains('/') && !line.contains('.') {
            current_author = line.to_string();
        } else if !current_author.is_empty() {
            file_authors
                .entry(line.to_string())
                .or_default()
                .insert(current_author.clone());
        }
    }
    file_authors
}

fn check_test_ratio(files: &[PathBuf]) -> Vec<Finding> {
    let (mut test_lines, mut prod_lines) = (0usize, 0usize);
    for f in files {
        let lines = std::fs::read_to_string(f)
            .map(|c| c.lines().count())
            .unwrap_or(0);
        if f.to_string_lossy().contains("test") || f.to_string_lossy().contains("spec") {
            test_lines += lines;
        } else {
            prod_lines += lines;
        }
    }
    if prod_lines == 0 || (test_lines as f64 / prod_lines as f64) >= 0.5 {
        return vec![];
    }
    let ratio = test_lines as f64 / prod_lines as f64;
    vec![Finding {
        smell_name: "low_test_ratio".into(),
        category: cha_core::SmellCategory::Dispensables,
        severity: cha_core::Severity::Hint,
        location: cha_core::Location {
            path: PathBuf::from("."),
            start_line: 1,
            end_line: 1,
            name: None,
        },
        message: format!(
            "Test-to-code ratio is {:.0}% ({test_lines} test / {prod_lines} production lines)",
            ratio * 100.0
        ),
        suggested_refactorings: vec!["Add unit tests".into()],
    }]
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
