use std::path::PathBuf;

/// Raw source file content.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
}

impl SourceFile {
    pub fn new(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }

    pub fn line_count(&self) -> usize {
        self.content.lines().count()
    }
}
