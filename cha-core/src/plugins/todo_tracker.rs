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
        ctx.model
            .comments
            .iter()
            .filter_map(|c| check_comment(c, ctx))
            .collect()
    }
}

fn check_comment(c: &crate::CommentInfo, ctx: &AnalysisContext) -> Option<Finding> {
    let upper = c.text.to_uppercase();
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
            start_line: c.line,
            end_line: c.line,
            name: None,
        },
        message: format!(
            "{tag}: {}",
            c.text.trim_start_matches(['/', '#', '*', ' ', '-'])
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
