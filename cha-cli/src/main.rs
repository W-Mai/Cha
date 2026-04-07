use std::path::PathBuf;

use cha_core::plugins::{ComplexityAnalyzer, LengthAnalyzer};
use cha_core::{AnalysisContext, Finding, Plugin, SourceFile};
use clap::Parser;

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
        Cli::Analyze { paths } => cmd_analyze(&paths),
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

fn cmd_analyze(paths: &[String]) {
    let plugins: Vec<Box<dyn Plugin>> = vec![
        Box::new(LengthAnalyzer::default()),
        Box::new(ComplexityAnalyzer::default()),
    ];
    let files = collect_files(paths);
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
        for plugin in &plugins {
            all_findings.extend(plugin.analyze(&ctx));
        }
    }

    if all_findings.is_empty() {
        println!("No issues found.");
    } else {
        for f in &all_findings {
            let icon = match f.severity {
                cha_core::Severity::Error => "✗",
                cha_core::Severity::Warning => "⚠",
                cha_core::Severity::Hint => "ℹ",
            };
            println!(
                "{} [{}] {}:{}-{} {}",
                icon,
                f.smell_name,
                f.location.path.display(),
                f.location.start_line,
                f.location.end_line,
                f.message,
            );
            if !f.suggested_refactorings.is_empty() {
                println!("  → suggested: {}", f.suggested_refactorings.join(", "));
            }
        }
        println!("\n{} issue(s) found.", all_findings.len());
    }
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
            "    - {} (L{}-L{}, {} lines)",
            f.name, f.start_line, f.end_line, f.line_count
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
