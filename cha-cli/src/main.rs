use std::path::PathBuf;
use std::process;

use cha_core::{
    AnalysisContext, Config, Finding, JsonReporter, LlmContextReporter, PluginRegistry, Reporter,
    SarifReporter, Severity, SourceFile, TerminalReporter,
};
use clap::{Parser, ValueEnum};
use rayon::prelude::*;

#[derive(Clone, ValueEnum)]
enum Format {
    Terminal,
    Json,
    Llm,
    Sarif,
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
    },
    /// Parse source files and show structure
    Parse {
        /// Files or directories to parse (defaults to current directory)
        paths: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Analyze {
            paths,
            format,
            fail_on,
            diff,
        } => {
            let code = cmd_analyze(&paths, &format, fail_on.as_ref(), diff);
            process::exit(code);
        }
        Cli::Parse { paths } => cmd_parse(&paths),
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
            continue;
        }
        // Recursive walk with .gitignore support
        let walker = ignore::WalkBuilder::new(&path)
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
    files
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

fn cmd_analyze(paths: &[String], format: &Format, fail_on: Option<&FailLevel>, diff: bool) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let config = Config::load(&cwd);
    let registry = PluginRegistry::from_config(&config, &cwd);
    let files = resolve_files(paths, diff);

    if files.is_empty() {
        println!("No files to analyze.");
        return 0;
    }

    let all_findings = run_analysis(&files, &registry);
    print_report(&all_findings, format);
    exit_code(&all_findings, fail_on)
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

/// Analyze files in parallel using rayon.
fn run_analysis(files: &[PathBuf], registry: &PluginRegistry) -> Vec<Finding> {
    files
        .par_iter()
        .flat_map(|path| {
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => return vec![],
            };
            let file = SourceFile::new(path.clone(), content);
            let model = match cha_parser::parse_file(&file) {
                Some(m) => m,
                None => return vec![],
            };
            let ctx = AnalysisContext {
                file: &file,
                model: &model,
            };
            registry
                .plugins()
                .iter()
                .flat_map(|p| p.analyze(&ctx))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn print_report(findings: &[Finding], format: &Format) {
    let reporter: Box<dyn Reporter> = match format {
        Format::Terminal => Box::new(TerminalReporter),
        Format::Json => Box::new(JsonReporter),
        Format::Llm => Box::new(LlmContextReporter),
        Format::Sarif => Box::new(SarifReporter),
    };
    println!("{}", reporter.render(findings));
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
