use std::path::PathBuf;

use cha_core::SourceFile;
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
        Cli::Analyze { paths } => {
            println!("analyze: {:?} (not yet implemented)", paths);
        }
        Cli::Parse { paths } => cmd_parse(&paths),
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
