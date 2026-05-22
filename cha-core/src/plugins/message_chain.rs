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
        let chains = collect_chains(ctx);
        ctx.model
            .functions
            .iter()
            .filter(|f| f.chain_depth > self.max_depth)
            .map(|f| {
                let loc = first_chain_in_range(&chains, f.start_line, f.end_line).unwrap_or((
                    f.start_line,
                    f.name_col,
                    f.name_end_col,
                ));
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
                    risk_score: None,
                }
            })
            .collect()
    }
}

/// Returns `(line, start_col, end_col)` per dotted-access node, sorted by file order.
fn collect_chains(ctx: &AnalysisContext) -> Vec<(usize, usize, usize)> {
    let (Some(tree), Some(lang)) = (ctx.tree, ctx.ts_language) else {
        return Vec::new();
    };
    let source = ctx.file.content.as_bytes();
    let patterns: &[&str] = match ctx.model.language.as_str() {
        "rust" => &["(field_expression) @c"],
        "typescript" | "javascript" => &["(member_expression) @c"],
        "python" => &["(attribute) @c"],
        "go" => &["(selector_expression) @c"],
        "c" | "cpp" => &["(field_expression) @c"],
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for pat in patterns {
        for matches in crate::query::run_query(tree, lang, source, pat) {
            for cap in matches {
                out.push((
                    cap.start_line as usize,
                    cap.start_col as usize,
                    cap.end_col as usize,
                ));
            }
        }
    }
    out.sort();
    out
}

fn first_chain_in_range(
    chains: &[(usize, usize, usize)],
    start: usize,
    end: usize,
) -> Option<(usize, usize, usize)> {
    chains
        .iter()
        .find(|(line, _, _)| *line >= start && *line <= end)
        .copied()
}
