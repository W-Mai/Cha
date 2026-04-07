use std::path::PathBuf;
use std::process;

use cha_core::{
    AnalysisContext, Config, Finding, JsonReporter, LlmContextReporter, PluginRegistry, Reporter,
    SarifReporter, Severity, SourceFile, TerminalReporter,
};
use clap::{Parser, ValueEnum};

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
        /// Files or directories to analyze
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
        /// Files to parse
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

fn collect_files(paths: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for p in paths {
        let path = PathBuf::from(p);
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    if ep.is_file() {
                        files.push(ep);
                    }
                }
            }
        } else {
            files.push(path);
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

    let files = if diff {
        let diff_files = git_diff_files();
        if diff_files.is_empty() && paths.is_empty() {
            println!("No changed files to analyze.");
            return 0;
        }
        if diff_files.is_empty() {
            collect_files(paths)
        } else {
            diff_files
        }
    } else {
        collect_files(paths)
    };

    let mut all_findings: Vec<Finding> = Vec::new();

    for path in &files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error reading {}: {}", path.display(), e);
                continue;
            }
        };
        let file = SourceFile::new(path.clone(), content);
        let model = match cha_parser::parse_file(&file) {
            Some(m) => m,
            None => continue,
        };
        let ctx = AnalysisContext {
            file: &file,
            model: &model,
        };
        for plugin in registry.plugins() {
            all_findings.extend(plugin.analyze(&ctx));
        }
    }

    let reporter: Box<dyn Reporter> = match format {
        Format::Terminal => Box::new(TerminalReporter),
        Format::Json => Box::new(JsonReporter),
        Format::Llm => Box::new(LlmContextReporter),
        Format::Sarif => Box::new(SarifReporter),
    };
    println!("{}", reporter.render(&all_findings));

    // Determine exit code
    if let Some(level) = fail_on {
        let threshold = match level {
            FailLevel::Hint => Severity::Hint,
            FailLevel::Warning => Severity::Warning,
            FailLevel::Error => Severity::Error,
        };
        if all_findings.iter().any(|f| f.severity >= threshold) {
            return 1;
        }
    }

    0
}

fn cmd_parse(paths: &[String]) {
    for path_str in paths {
        let path = PathBuf::from(path_str);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error reading {}: {}", path_str, e);
                continue;
            }
        };
        let file = SourceFile::new(path, content);
        match cha_parser::parse_file(&file) {
            Some(model) => print_model(path_str, &model),
            None => eprintln!("unsupported file: {}", path_str),
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
