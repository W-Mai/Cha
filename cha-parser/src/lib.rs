mod rust_lang;
mod typescript;

pub use cha_core::{ClassInfo, FunctionInfo, ImportInfo, SourceModel};
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
        _ => return None,
    };
    parser.parse(file)
}
