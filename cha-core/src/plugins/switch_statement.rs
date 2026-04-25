use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect functions with excessive switch/match arms.
pub struct SwitchStatementAnalyzer {
    pub max_arms: usize,
}

impl Default for SwitchStatementAnalyzer {
    fn default() -> Self {
        Self { max_arms: 8 }
    }
}

impl Plugin for SwitchStatementAnalyzer {
    fn name(&self) -> &str {
        "switch_statement"
    }

    fn smells(&self) -> Vec<String> {
        vec!["switch_statement".into()]
    }

    fn description(&self) -> &str {
        "Excessive switch/match arms"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let lines: Vec<&str> = ctx.file.content.lines().collect();
        ctx.model
            .functions
            .iter()
            .filter(|f| f.switch_arms > self.max_arms)
            .map(|f| {
                let loc = find_switch_keyword(&lines, f.start_line, f.end_line).unwrap_or((
                    f.start_line,
                    f.name_col,
                    f.name_end_col,
                ));
                Finding {
                    smell_name: "switch_statement".into(),
                    category: SmellCategory::OoAbusers,
                    severity: Severity::Warning,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: loc.0,
                        start_col: loc.1,
                        end_line: loc.0,
                        end_col: loc.2,
                        name: Some(f.name.clone()),
                    },
                    message: format!(
                        "Function `{}` has {} switch/match arms (threshold: {})",
                        f.name, f.switch_arms, self.max_arms
                    ),
                    suggested_refactorings: vec!["Replace Conditional with Polymorphism".into()],
                    actual_value: Some(f.switch_arms as f64),
                    threshold: Some(self.max_arms as f64),
                    risk_score: None,
                }
            })
            .collect()
    }
}

/// Scan the function body for the first `switch`/`match` keyword and return
/// `(line, start_col, end_col)` of the keyword token. Returns None if not
/// found (fallback to function name location).
fn find_switch_keyword(lines: &[&str], start: usize, end: usize) -> Option<(usize, usize, usize)> {
    let keywords = ["switch", "match"];
    for (idx, line) in lines
        .iter()
        .enumerate()
        .take(end.min(lines.len()))
        .skip(start.saturating_sub(1))
    {
        for kw in &keywords {
            if let Some(col) = find_keyword(line, kw) {
                return Some((idx + 1, col, col + kw.len()));
            }
        }
    }
    None
}

/// Find `keyword` in `line` but only where it stands as a word (whitespace /
/// line start before, non-alphanumeric after). Skips occurrences inside
/// comments starting at line start (`//`, `#`, `/*`).
fn find_keyword(line: &str, keyword: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
        return None;
    }
    let bytes = line.as_bytes();
    let klen = keyword.len();
    let mut i = 0;
    while i + klen <= bytes.len() {
        if &bytes[i..i + klen] == keyword.as_bytes() {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let after_ok = i + klen == bytes.len() || !is_ident_byte(bytes[i + klen]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
