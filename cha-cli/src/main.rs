use std::path::{Path, PathBuf};
use std::process;

mod analyze;
mod deps;
mod diff;
mod hotspot;
mod plugin;
mod tangled;
mod trend;

use cha_core::{Config, Finding, SourceFile};
use clap::{CommandFactory, Parser, ValueEnum};

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

#[derive(Clone, ValueEnum)]
pub(crate) enum DepsFormat {
    Dot,
    Json,
    Mermaid,
}

#[derive(Clone, ValueEnum)]
pub(crate) enum DepsDepth {
    File,
    Dir,
}

#[derive(Clone, ValueEnum, Default)]
pub(crate) enum DepsType {
    #[default]
    Imports,
    Classes,
    Calls,
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
    /// Show dependency graph (imports, classes, or calls)
    Deps {
        /// Files or directories (defaults to current directory)
        paths: Vec<String>,
        /// Output format
        #[arg(long, default_value = "dot")]
        format: DepsFormat,
        /// Aggregation depth: "file" (default) or "dir" (imports only)
        #[arg(long, default_value = "file")]
        depth: DepsDepth,
        /// Graph type: imports (default), classes, or calls
        #[arg(long, default_value = "imports")]
        r#type: DepsType,
        /// Filter to specific class/function name (shows connected subgraph)
        #[arg(long)]
        filter: Option<String>,
        /// Exact match: only show edges directly involving the filter name
        #[arg(long)]
        exact: bool,
    },
    /// Analyze recent git commits to show issue trend
    Trend {
        /// Number of commits to analyze (default: 10)
        #[arg(short, long, default_value = "10")]
        count: usize,
        /// Output format (terminal or json)
        #[arg(long, default_value = "terminal")]
        format: Format,
    },
    /// Show hotspots: files with high change frequency × complexity
    Hotspot {
        /// Number of recent commits to analyze (default: 100)
        #[arg(short, long, default_value = "100")]
        count: usize,
        /// Show top N files (default: 20)
        #[arg(short, long, default_value = "20")]
        top: usize,
        /// Output format (terminal or json)
        #[arg(long, default_value = "terminal")]
        format: Format,
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
    let code = dispatch(cli);
    if code != 0 {
        process::exit(code);
    }
}

fn dispatch(cli: Cli) -> i32 {
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
            analyze::cmd_analyze(
                &paths,
                &format,
                fail_on.as_ref(),
                mode,
                &plugin,
                baseline.as_deref(),
                output.as_deref(),
            )
        }
        other => {
            run_other(other);
            0
        }
    }
}

// cha:ignore switch_statement
fn run_other(cli: Cli) {
    match cli {
        Cli::Baseline { paths, output } => cmd_baseline(&paths, output.as_deref()),
        Cli::Parse { paths } => cmd_parse(&paths),
        Cli::Init | Cli::Schema => cmd_init_or_schema(cli),
        Cli::Fix {
            paths,
            diff,
            dry_run,
        } => cmd_fix(&paths, diff, dry_run),
        Cli::Plugin { cmd } => cmd_plugin(cmd),
        Cli::Trend { count, format } => trend::cmd_trend(count, &format),
        Cli::Hotspot { count, top, format } => hotspot::cmd_hotspot(count, top, &format),
        Cli::Deps {
            paths,
            format,
            depth,
            r#type,
            filter,
            exact,
        } => deps::cmd_deps(&paths, &format, &depth, &r#type, filter.as_deref(), exact),
        Cli::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "cha", &mut std::io::stdout());
        }
        _ => unreachable!(),
    }
}

fn cmd_init_or_schema(cli: Cli) {
    match cli {
        Cli::Init => cmd_init(),
        Cli::Schema => println!("{}", cha_core::findings_json_schema()),
        _ => unreachable!(),
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

pub(crate) fn new_progress_bar(len: u64) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new(len);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"])
            .progress_chars("█▓░"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Recursively collect source files, respecting .gitignore and skipping common build dirs.
pub(crate) fn collect_files(paths: &[String]) -> Vec<PathBuf> {
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
pub(crate) fn git_diff_files() -> Vec<PathBuf> {
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

fn cmd_baseline(paths: &[String], output: Option<&str>) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let root_config = Config::load(&cwd);
    let files = analyze::filter_excluded(collect_files(paths), &root_config.exclude, &cwd);
    let findings = analyze::run_analysis(&files, &cwd, &[]);
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
    let files = analyze::resolve_files(paths, diff);
    if files.is_empty() {
        println!("No files to fix.");
        return;
    }
    let project_root = std::env::current_dir().unwrap_or_default();
    let filter = vec!["naming".to_string()];
    let findings = analyze::run_analysis(&files, &project_root, &filter);

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
