use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

const PRIMITIVE_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char", "String", "&str", "string", "number", "boolean", "any",
];

/// Detect functions where most parameters are primitive types.
pub struct PrimitiveObsessionAnalyzer {
    pub min_params: usize,
    pub primitive_ratio: f64,
}

impl Default for PrimitiveObsessionAnalyzer {
    fn default() -> Self {
        Self {
            min_params: 3,
            primitive_ratio: 0.8,
        }
    }
}

impl Plugin for PrimitiveObsessionAnalyzer {
    fn name(&self) -> &str {
        "primitive_obsession"
    }

    fn smells(&self) -> Vec<String> {
        vec!["primitive_obsession".into()]
    }

    fn description(&self) -> &str {
        "Too many primitive parameter types"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter_map(|f| {
                let total = f.parameter_types.len();
                if total < self.min_params {
                    return None;
                }
                let prim_count = f
                    .parameter_types
                    .iter()
                    .filter(|t| is_primitive(&t.name))
                    .count();
                let ratio = prim_count as f64 / total as f64;
                if ratio < self.primitive_ratio {
                    return None;
                }
                Some(Finding {
                    smell_name: "primitive_obsession".into(),
                    category: SmellCategory::Bloaters,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: f.start_line,
                        start_col: f.name_col,
                        end_line: f.start_line,
                        end_col: f.name_end_col,
                        name: Some(f.name.clone()),
                    },
                    message: format!(
                        "Function `{}` uses mostly primitive parameter types",
                        f.name
                    ),
                    suggested_refactorings: vec![
                        "Replace Data Value with Object".into(),
                        "Replace Type Code with Class".into(),
                    ],
                    actual_value: Some(ratio),
                    threshold: Some(self.primitive_ratio),
                    risk_score: None,
                })
            })
            .collect()
    }
}

fn is_primitive(ty: &str) -> bool {
    let base = ty.trim_start_matches('&').trim_start_matches("mut ").trim();
    // Strip generic parameters: Vec<String> → Vec
    let base = base.split('<').next().unwrap_or(base);
    PRIMITIVE_TYPES.contains(&base)
}
