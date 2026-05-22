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
        let switch_nodes = collect_switch_nodes(ctx);
        ctx.model
            .functions
            .iter()
            .filter(|f| f.switch_arms > self.max_arms)
            .map(|f| {
                let loc = first_switch_in_range(&switch_nodes, f.start_line, f.end_line)
                    .unwrap_or((f.start_line, f.name_col, f.name_end_col));
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

/// Returns `(line, start_col, end_col)` per switch/match, sorted by file order.
fn collect_switch_nodes(ctx: &AnalysisContext) -> Vec<(usize, usize, usize)> {
    let (Some(tree), Some(lang)) = (ctx.tree, ctx.ts_language) else {
        return Vec::new();
    };
    let source = ctx.file.content.as_bytes();
    let patterns: &[&str] = match ctx.model.language.as_str() {
        "rust" => &["(match_expression) @s"],
        "typescript" => &["(switch_statement) @s"],
        "python" => &["(match_statement) @s"],
        "go" => &[
            "(expression_switch_statement) @s",
            "(type_switch_statement) @s",
        ],
        "c" | "cpp" => &["(switch_statement) @s"],
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for pat in patterns {
        for matches in crate::query::run_query(tree, lang, source, pat) {
            for cap in matches {
                let len = match cap.node_kind.as_str() {
                    "match_expression" | "match_statement" => "match".len(),
                    _ => "switch".len(),
                };
                out.push((
                    cap.start_line as usize,
                    cap.start_col as usize,
                    cap.start_col as usize + len,
                ));
            }
        }
    }
    out.sort();
    out
}

fn first_switch_in_range(
    switches: &[(usize, usize, usize)],
    start: usize,
    end: usize,
) -> Option<(usize, usize, usize)> {
    switches
        .iter()
        .find(|(line, _, _)| *line >= start && *line <= end)
        .copied()
}
