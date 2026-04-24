mod c_imports;
mod c_lang;
mod golang;
mod golang_imports;
mod python;
mod python_imports;
mod rust_imports;
mod rust_lang;
mod type_ref;
mod typescript;
mod typescript_imports;

pub use c_lang::{CParser, CppParser};
pub use cha_core::{ClassInfo, CommentInfo, FunctionInfo, ImportInfo, SourceModel};
pub use golang::GolangParser;
pub use python::PythonParser;
pub use rust_lang::RustParser;
pub use typescript::TypeScriptParser;

use cha_core::SourceFile;

/// Trait for language-specific parsers.
pub trait LanguageParser: Send + Sync {
    fn language_name(&self) -> &str;
    fn parse(&self, file: &SourceFile) -> Option<SourceModel>;
}

/// Detect language from file extension and parse.
pub fn parse_file(file: &SourceFile) -> Option<SourceModel> {
    let ext = file.path.extension()?.to_str()?;
    let parser: Box<dyn LanguageParser> = match ext {
        "ts" | "tsx" => Box::new(TypeScriptParser),
        "rs" => Box::new(RustParser),
        "py" => Box::new(PythonParser),
        "go" => Box::new(GolangParser),
        "h" if looks_like_cpp(&file.content) => Box::new(CppParser),
        "c" | "h" => Box::new(CParser),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Box::new(CppParser),
        _ => return None,
    };
    parser.parse(file)
}

/// Sniff whether a `.h` file contains C++ constructs.
fn looks_like_cpp(content: &str) -> bool {
    content.lines().any(|line| {
        let t = line.trim();
        t.starts_with("class ")
            || t.starts_with("namespace ")
            || t.starts_with("template")
            || t.starts_with("using ")
            || t.contains("public:")
            || t.contains("private:")
            || t.contains("protected:")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::path::PathBuf;

    proptest! {
        #[test]
        fn parse_rust_never_panics(content in ".*") {
            let file = SourceFile::new(PathBuf::from("test.rs"), content);
            let _ = parse_file(&file);
        }

        #[test]
        fn parse_ts_never_panics(content in ".*") {
            let file = SourceFile::new(PathBuf::from("test.ts"), content);
            let _ = parse_file(&file);
        }

        #[test]
        fn parse_unknown_ext_returns_none(content in ".*") {
            let file = SourceFile::new(PathBuf::from("test.txt"), content);
            prop_assert!(parse_file(&file).is_none());
        }

        #[test]
        fn parse_model_invariants(content in ".{0,500}") {
            let file = SourceFile::new(PathBuf::from("test.rs"), content.clone());
            if let Some(model) = parse_file(&file) {
                prop_assert_eq!(model.language, "rust");
                prop_assert!(model.total_lines > 0 || content.is_empty());
                for f in &model.functions {
                    prop_assert!(f.start_line <= f.end_line);
                    prop_assert!(f.line_count > 0);
                    prop_assert!(!f.name.is_empty());
                }
                for c in &model.classes {
                    prop_assert!(c.start_line <= c.end_line);
                    prop_assert!(!c.name.is_empty());
                }
            }
        }
    }
}
