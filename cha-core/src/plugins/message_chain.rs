use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect deep method chain calls (e.g. a.b().c().d()).
pub struct MessageChainAnalyzer {
    pub max_depth: usize,
}

impl Default for MessageChainAnalyzer {
    fn default() -> Self {
        Self { max_depth: 3 }
    }
}

impl Plugin for MessageChainAnalyzer {
    fn name(&self) -> &str {
        "message_chain"
    }

    fn smells(&self) -> Vec<String> {
        vec!["message_chain".into()]
    }

    fn description(&self) -> &str {
        "Deep field access chains (a.b.c.d)"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let lines: Vec<&str> = ctx.file.content.lines().collect();
        ctx.model
            .functions
            .iter()
            .filter(|f| f.chain_depth > self.max_depth)
            .map(|f| {
                let loc = find_deepest_chain(&lines, f.start_line, f.end_line, self.max_depth)
                    .unwrap_or((f.start_line, f.name_col, f.name_end_col));
                Finding {
                    smell_name: "message_chain".into(),
                    category: SmellCategory::Couplers,
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
                        "Function `{}` has chain depth {} (threshold: {})",
                        f.name, f.chain_depth, self.max_depth
                    ),
                    suggested_refactorings: vec!["Hide Delegate".into()],
                    actual_value: Some(f.chain_depth as f64),
                    threshold: Some(self.max_depth as f64),
                }
            })
            .collect()
    }
}

/// Scan function body for the first line where a dot-chain reaches at least
/// `min_depth + 1` segments (matching the parser's `chain_depth > max_depth`
/// semantics). Return `(line, start_col, end_col)` covering the chain.
fn find_deepest_chain(
    lines: &[&str],
    start: usize,
    end: usize,
    min_depth: usize,
) -> Option<(usize, usize, usize)> {
    for (idx, line) in lines
        .iter()
        .enumerate()
        .take(end.min(lines.len()))
        .skip(start.saturating_sub(1))
    {
        if let Some((col, chain_end)) = longest_chain(line, min_depth + 1) {
            return Some((idx + 1, col, chain_end));
        }
    }
    None
}

/// Return `(start_col, end_col)` of the first chain on `line` that has at
/// least `min_segments` identifier segments separated by dots. Skips comment
/// lines.
fn longest_chain(line: &str, min_segments: usize) -> Option<(usize, usize)> {
    if is_comment_line(line) {
        return None;
    }
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if is_chain_start(bytes, i) {
            let (end, segments) = walk_chain(bytes, i);
            if segments >= min_segments {
                return Some((i, end));
            }
            i = end.max(i + 1);
        } else {
            i += 1;
        }
    }
    None
}

fn is_comment_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with('#') || t.starts_with("/*")
}

fn is_chain_start(bytes: &[u8], i: usize) -> bool {
    is_ident_start(bytes[i]) && !bytes[i].is_ascii_digit()
}

/// Walk one chain starting at `i`. Returns `(end_col, segment_count)`.
fn walk_chain(bytes: &[u8], start: usize) -> (usize, usize) {
    let mut cur = start;
    let mut segments = 0;
    loop {
        cur = advance_ident(bytes, cur);
        segments += 1;
        cur = skip_optional_call(bytes, cur);
        if !advance_dot_if_chain(bytes, &mut cur) {
            break;
        }
    }
    (cur, segments)
}

fn advance_ident(bytes: &[u8], from: usize) -> usize {
    let mut cur = from;
    while cur < bytes.len() && is_ident_cont(bytes[cur]) {
        cur += 1;
    }
    cur
}

fn skip_optional_call(bytes: &[u8], from: usize) -> usize {
    if from < bytes.len()
        && bytes[from] == b'('
        && let Some(e) = match_paren_end(bytes, from)
    {
        return e + 1;
    }
    from
}

/// If `bytes[*cur]` is `.` followed by an identifier, advance past the dot
/// and return true (continue chain). Otherwise leave `cur` and return false.
fn advance_dot_if_chain(bytes: &[u8], cur: &mut usize) -> bool {
    if *cur < bytes.len()
        && bytes[*cur] == b'.'
        && *cur + 1 < bytes.len()
        && is_ident_start(bytes[*cur + 1])
    {
        *cur += 1;
        return true;
    }
    false
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn match_paren_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}
