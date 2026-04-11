use std::path::{Path, PathBuf};
use std::process;

mod diff;
mod plugin;

use cha_core::{
    AnalysisContext, Config, Finding, JsonReporter, LlmContextReporter, PluginRegistry, Reporter,
    SarifReporter, Severity, SourceFile, TerminalReporter,
};
use clap::{CommandFactory, Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

#[derive(Clone, ValueEnum)]
enum Format {
    Terminal,
    Json,
    Llm,
    Sarif,
    Html,
}

#[derive(Clone, ValueEnum, PartialEq, Eq, PartialOrd, Ord)]
enum FailLevel {
    Hint,
    Warning,
    Error,
}

#[derive(Parser)]
#[command(
    name = "cha",
    version,
    about = "察 — Code quality & architecture analysis engine"
)]
enum Cli {
    /// Analyze source files for code smells
    Analyze {
        /// Files or directories to analyze (defaults to current directory)
        paths: Vec<String>,
        /// Output format
        #[arg(long, default_value = "terminal")]
        format: Format,
        /// Exit with code 1 if findings at this severity or above exist
        #[arg(long)]
        fail_on: Option<FailLevel>,
        /// Only analyze files changed in git diff (unstaged)
        #[arg(long)]
        diff: bool,
        /// Read unified diff from stdin, analyze only changed files/lines
        #[arg(long)]
        stdin_diff: bool,
        /// Only run specific plugins (comma-separated names)
        #[arg(long, value_delimiter = ',')]
        plugin: Vec<String>,
        /// Disable analysis cache (force full re-analysis)
        #[arg(long)]
        no_cache: bool,
        /// Only report findings not in the baseline file
        #[arg(long)]
        baseline: Option<String>,
        /// Write output to file (used with --format html)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Generate a baseline file from current findings (suppresses known issues)
    Baseline {
        /// Files or directories to analyze (defaults to current directory)
        paths: Vec<String>,
        /// Output path for baseline file (default: .cha/baseline.json)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Parse source files and show structure
    Parse {
        /// Files or directories to parse (defaults to current directory)
        paths: Vec<String>,
    },
    /// Generate a default .cha.toml configuration file
    Init,
    /// Print JSON Schema for the analysis output format
    Schema,
    /// Auto-fix simple issues (naming conventions)
    Fix {
        /// Files or directories to fix (defaults to current directory)
        paths: Vec<String>,
        /// Only fix files changed in git diff (unstaged)
        #[arg(long)]
        diff: bool,
        /// Dry run — show what would be changed without modifying files
        #[arg(long)]
        dry_run: bool,
    },
    /// Manage WASM plugins
    Plugin {
        #[command(subcommand)]
        cmd: PluginCmd,
    },
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
}

#[derive(clap::Subcommand)]
enum PluginCmd {
    /// Scaffold a new plugin project
    New {
        /// Plugin name
        name: String,
    },
    /// Build the plugin in the current directory
    Build,
    /// List installed plugins
    List,
    /// Install a .wasm file into .cha/plugins/
    Install {
        /// Path to the .wasm file
        path: String,
    },
    /// Remove an installed plugin
    Remove {
        /// Plugin name (with or without .wasm extension)
        name: String,
    },
}

impl DiffMode {
    fn from_flags(diff: bool, stdin_diff: bool) -> Self {
        if stdin_diff {
            Self::Stdin
        } else if diff {
            Self::Git
        } else {
            Self::None
        }
    }
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Analyze {
            paths,
            format,
            fail_on,
            diff,
            stdin_diff,
            plugin,
            no_cache,
            baseline,
            output,
        } => {
            let mode = DiffMode::from_flags(diff, stdin_diff);
            if no_cache {
                let cwd = std::env::current_dir().unwrap_or_default();
                let _ = std::fs::remove_dir_all(cwd.join(".cha/cache"));
            }
            let code = cmd_analyze(
                &paths,
                &format,
                fail_on.as_ref(),
                mode,
                &plugin,
                baseline.as_deref(),
                output.as_deref(),
            );
            process::exit(code);
        }
        Cli::Baseline { paths, output } => cmd_baseline(&paths, output.as_deref()),
        Cli::Parse { paths } => cmd_parse(&paths),
        Cli::Init => cmd_init(),
        Cli::Schema => println!("{}", cha_core::findings_json_schema()),
        Cli::Fix {
            paths,
            diff,
            dry_run,
        } => cmd_fix(&paths, diff, dry_run),
        Cli::Plugin { cmd } => cmd_plugin(cmd),
        Cli::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "cha", &mut std::io::stdout());
        }
    }
}

fn cmd_plugin(cmd: PluginCmd) {
    match cmd {
        PluginCmd::New { name } => plugin::cmd_new(&name),
        PluginCmd::Build => plugin::cmd_build(),
        PluginCmd::List => plugin::cmd_list(),
        PluginCmd::Install { path } => plugin::cmd_install(&path),
        PluginCmd::Remove { name } => plugin::cmd_remove(&name),
    }
}

/// Recursively collect source files, respecting .gitignore and skipping common build dirs.
fn collect_files(paths: &[String]) -> Vec<PathBuf> {
    let targets: Vec<&str> = if paths.is_empty() {
        vec!["."]
    } else {
        paths.iter().map(|s| s.as_str()).collect()
    };

    let mut files = Vec::new();
    for target in targets {
        let path = PathBuf::from(target);
        if path.is_file() {
            files.push(path);
        } else {
            walk_directory(&path, &mut files);
        }
    }
    files
}

// Walk a directory recursively, collecting files while respecting .gitignore.
fn walk_directory(root: &Path, files: &mut Vec<PathBuf>) {
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !matches!(name.as_ref(), "target" | "node_modules" | "dist" | "build")
        })
        .build();
    for entry in walker.flatten() {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            files.push(entry.into_path());
        }
    }
}

/// Get changed files from git diff.
fn git_diff_files() -> Vec<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=ACMR", "HEAD"])
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(PathBuf::from)
            .collect(),
        _ => {
            eprintln!("warning: git diff failed, analyzing all files");
            vec![]
        }
    }
}

/// How to handle diff-based filtering.
enum DiffMode {
    None,
    Git,
    Stdin,
}

fn cmd_analyze(
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
fn filter_excluded(files: Vec<PathBuf>, patterns: &[String], root: &Path) -> Vec<PathBuf> {
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

fn resolve_files(paths: &[String], diff: bool) -> Vec<PathBuf> {
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
fn run_analysis(files: &[PathBuf], project_root: &Path, plugin_filter: &[String]) -> Vec<Finding> {
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

fn print_report(
    findings: &[Finding],
    format: &Format,
    files: &[PathBuf],
    weights: &cha_core::DebtWeights,
) {
    match format {
        Format::Json | Format::Sarif => {
            let file_lines: Vec<(String, usize)> = files
                .iter()
                .filter_map(|p| {
                    let c = std::fs::read_to_string(p).ok()?;
                    Some((p.to_string_lossy().to_string(), c.lines().count()))
                })
                .collect();
            let scores = cha_core::score_files(findings, &file_lines, weights);
            let output = match format {
                Format::Json => JsonReporter.render_with_scores(findings, &scores),
                Format::Sarif => SarifReporter.render_with_scores(findings, &scores),
                _ => unreachable!(),
            };
            println!("{output}");
        }
        _ => {
            let reporter: Box<dyn Reporter> = match format {
                Format::Terminal => Box::new(TerminalReporter),
                Format::Llm => Box::new(LlmContextReporter),
                _ => unreachable!(),
            };
            println!("{}", reporter.render(findings));
        }
    }
    if matches!(format, Format::Terminal) && !findings.is_empty() {
        print_health_scores(findings, files, weights);
    }
}

fn print_health_scores(findings: &[Finding], files: &[PathBuf], weights: &cha_core::DebtWeights) {
    let file_lines: Vec<(String, usize)> = files
        .iter()
        .filter_map(|p| {
            let content = std::fs::read_to_string(p).ok()?;
            Some((p.to_string_lossy().to_string(), content.lines().count()))
        })
        .collect();
    let mut scores = cha_core::score_files(findings, &file_lines, weights);
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

fn exit_code(findings: &[Finding], fail_on: Option<&FailLevel>) -> i32 {
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

fn cmd_baseline(paths: &[String], output: Option<&str>) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = Config::load(&cwd);
    let files = filter_excluded(collect_files(paths), &root_config.exclude, &cwd);
    let findings = run_analysis(&files, &cwd, &[]);
    let baseline = cha_core::Baseline::from_findings(&findings, &cwd);
    let out = Path::new(output.unwrap_or(".cha/baseline.json"));
    match baseline.save(out) {
        Ok(()) => println!(
            "Baseline saved to {} ({} findings)",
            out.display(),
            baseline.fingerprints.len()
        ),
        Err(e) => eprintln!("Failed to save baseline: {e}"),
    }
}

fn cmd_parse(paths: &[String]) {
    let files = collect_files(paths);
    for path in &files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error reading {}: {}", path.display(), e);
                continue;
            }
        };
        let file = SourceFile::new(path.clone(), content);
        if let Some(model) = cha_parser::parse_file(&file) {
            print_model(&path.to_string_lossy(), &model);
        }
    }
}

fn print_model(path: &str, model: &cha_parser::SourceModel) {
    println!("=== {} ({}) ===", path, model.language);
    println!("  lines: {}", model.total_lines);
    println!("  functions: {}", model.functions.len());
    for f in &model.functions {
        println!(
            "    - {} (L{}-L{}, {} lines, complexity {})",
            f.name, f.start_line, f.end_line, f.line_count, f.complexity
        );
    }
    println!("  classes: {}", model.classes.len());
    for c in &model.classes {
        println!(
            "    - {} (L{}-L{}, {} methods, {} lines)",
            c.name, c.start_line, c.end_line, c.method_count, c.line_count
        );
    }
    println!("  imports: {}", model.imports.len());
    for i in &model.imports {
        println!("    - {} (L{})", i.source, i.line);
    }
}

fn cmd_init() {
    let path = Path::new(".cha.toml");
    if path.exists() {
        eprintln!(".cha.toml already exists");
        process::exit(1);
    }
    std::fs::write(path, include_str!("../../static/default.cha.toml"))
        .expect("failed to write .cha.toml");
    println!("Created .cha.toml");
}

fn cmd_fix(paths: &[String], diff: bool, dry_run: bool) {
    let files = resolve_files(paths, diff);
    if files.is_empty() {
        println!("No files to fix.");
        return;
    }
    let project_root = std::env::current_dir().unwrap_or_default();
    let filter = vec!["naming".to_string()];
    let findings = run_analysis(&files, &project_root, &filter);

    let fixable: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.smell_name == "naming_convention")
        .collect();

    if fixable.is_empty() {
        println!("Nothing to fix.");
        return;
    }

    let fixed: usize = fixable.iter().filter_map(|f| apply_fix(f, dry_run)).count();
    let label = if dry_run {
        "would be applied"
    } else {
        "applied"
    };
    println!("{fixed} fix(es) {label}.");
}

/// Apply a single naming convention fix. Returns Some(()) if applied.
fn apply_fix(finding: &Finding, dry_run: bool) -> Option<()> {
    let name = finding.location.name.as_ref()?;
    let new_name = to_pascal_case(name);
    if new_name == *name {
        return None;
    }
    let path = &finding.location.path;
    let content = std::fs::read_to_string(path).ok()?;
    let replaced = content.replace(name.as_str(), &new_name);
    if replaced == content {
        return None;
    }
    if dry_run {
        println!("  {name} → {new_name} in {}", path.display());
    } else {
        std::fs::write(path, &replaced).ok()?;
        println!("  Fixed: {name} → {new_name} in {}", path.display());
    }
    Some(())
}

/// Convert a name to PascalCase by uppercasing the first character.
fn to_pascal_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
