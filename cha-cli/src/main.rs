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
        Cli::Parse { paths } => {
            println!("parse: {:?} (not yet implemented)", paths);
        }
    }
}
