use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

// cha:ignore todo_comment
/// Detect leftover task comments (todo/fixme/hack/xxx) in source code.
///
/// Severity levels: H/X tags → Warning, F/T tags → Hint.
pub struct TodoTrackerAnalyzer;

impl Plugin for TodoTrackerAnalyzer {
    fn name(&self) -> &str {
        "todo_tracker"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.file
            .content
            .lines()
            .enumerate()
            .filter_map(|(i, line)| check_line(i + 1, line, ctx))
            .collect()
    }
}

// cha:ignore high_complexity,cognitive_complexity
fn check_line(line_num: usize, line: &str, ctx: &AnalysisContext) -> Option<Finding> {
    let trimmed = line.trim();
    // Check full-line comments or trailing comments
    let comment = if is_comment(trimmed) {
        trimmed
    } else if let Some(pos) = trimmed.find("//") {
        &trimmed[pos..]
    } else if let Some(pos) = trimmed.find('#') {
        &trimmed[pos..]
    } else {
        return None;
    };
    let upper = comment.to_uppercase();
    let (tag, severity) = if has_tag(&upper, "HACK") {
        ("HACK", Severity::Warning)
    } else if has_tag(&upper, "XXX") {
        ("XXX", Severity::Warning)
    } else if has_tag(&upper, "FIXME") {
        ("FIXME", Severity::Hint)
    } else if has_tag(&upper, "TODO") {
        ("TODO", Severity::Hint)
    } else {
        return None;
    };
    Some(Finding {
        smell_name: "todo_comment".into(),
        category: SmellCategory::Dispensables,
        severity,
        location: Location {
            path: ctx.file.path.clone(),
            start_line: line_num,
            end_line: line_num,
            name: None,
        },
        message: format!(
            "{tag}: {}",
            comment.trim_start_matches(['/', '#', '*', ' ', '-'])
        ),
        suggested_refactorings: vec!["Resolve or create a tracking issue".into()],
    })
}

/// Match tag as a word boundary (e.g. "TAG:" or "TAG " but not "TAGLIST")
fn has_tag(line: &str, tag: &str) -> bool {
    if let Some(pos) = line.find(tag) {
        let after = pos + tag.len();
        after >= line.len() || !line.as_bytes()[after].is_ascii_alphabetic()
    } else {
        false
    }
}

fn is_comment(line: &str) -> bool {
    line.starts_with("//")
        || line.starts_with('#')
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with("--")
}
