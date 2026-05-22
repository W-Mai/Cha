use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

// cha:ignore large_class
pub struct DesignPatternAdvisor {
    pub strategy_min_arms: usize,
    pub state_min_arms: usize,
    pub builder_min_params: usize,
    pub builder_alt_min_params: usize,
    pub builder_alt_min_optional: usize,
    pub null_object_min_count: usize,
    pub template_min_self_calls: usize,
    pub template_min_methods: usize,
    pub type_field_keywords: Vec<String>,
    pub state_field_keywords: Vec<String>,
}

impl Default for DesignPatternAdvisor {
    fn default() -> Self {
        Self {
            strategy_min_arms: 4,
            state_min_arms: 3,
            builder_min_params: 7,
            builder_alt_min_params: 5,
            builder_alt_min_optional: 3,
            null_object_min_count: 3,
            template_min_self_calls: 3,
            template_min_methods: 4,
            type_field_keywords: ["type", "kind", "role", "action", "mode"]
                .iter()
                .map(|s| (*s).into())
                .collect(),
            state_field_keywords: ["state", "status"].iter().map(|s| (*s).into()).collect(),
        }
    }
}

impl Plugin for DesignPatternAdvisor {
    fn name(&self) -> &str {
        "design_pattern"
    }

    fn smells(&self) -> Vec<String> {
        vec![
            "strategy_pattern".into(),
            "state_pattern".into(),
            "builder_pattern".into(),
            "null_object_pattern".into(),
            "template_method_pattern".into(),
            "observer_pattern".into(),
        ]
    }

    fn description(&self) -> &str {
        "Suggest design patterns based on code structure"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        self.check_strategy(ctx, &mut findings);
        self.check_state(ctx, &mut findings);
        self.check_builder(ctx, &mut findings);
        self.check_null_object(ctx, &mut findings);
        self.check_template_method(ctx, &mut findings);
        check_observer(ctx, &mut findings);
        findings
    }
}

impl DesignPatternAdvisor {
    fn matches_keyword(name: &str, keywords: &[String]) -> bool {
        let l = name.to_lowercase();
        keywords.iter().any(|k| l.contains(k.as_str()))
    }

    fn check_strategy(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for f in &ctx.model.functions {
            let target = match f.switch_dispatch_target.as_deref() {
                Some(t)
                    if f.switch_arms >= self.strategy_min_arms
                        && Self::matches_keyword(t, &self.type_field_keywords) =>
                {
                    t
                }
                _ => continue,
            };
            findings.push(hint(
                ctx,
                (f.start_line, f.name_col, f.name_end_col, Some(&f.name)),
                "strategy_pattern",
                format!(
                    "Function `{}` dispatches on `{}` with {} arms — consider Strategy pattern",
                    f.name, target, f.switch_arms
                ),
            ));
        }
    }

    fn check_state(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for f in &ctx.model.functions {
            let target = match f.switch_dispatch_target.as_deref() {
                Some(t)
                    if f.switch_arms >= self.state_min_arms
                        && Self::matches_keyword(t, &self.state_field_keywords) =>
                {
                    t
                }
                _ => continue,
            };
            findings.push(hint(
                ctx,
                (f.start_line, f.name_col, f.name_end_col, Some(&f.name)),
                "state_pattern",
                format!(
                    "Function `{}` dispatches on `{}` — consider State pattern",
                    f.name, target
                ),
            ));
        }
    }

    fn check_builder(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for f in &ctx.model.functions {
            if f.parameter_count >= self.builder_min_params
                || (f.parameter_count >= self.builder_alt_min_params
                    && f.optional_param_count >= self.builder_alt_min_optional)
            {
                findings.push(hint(
                    ctx,
                    (f.start_line, f.name_col, f.name_end_col, Some(&f.name)),
                    "builder_pattern",
                    format!(
                        "Function `{}` has {} params ({} optional) — consider Builder pattern",
                        f.name, f.parameter_count, f.optional_param_count
                    ),
                ));
            }
        }
    }

    fn check_null_object(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for f in &ctx.model.functions {
            for field in &f.null_check_fields {
                *counts.entry(field).or_default() += 1;
            }
        }
        for (field, count) in &counts {
            if *count >= self.null_object_min_count {
                findings.push(hint(
                    ctx,
                    (1, 0, 0, None),
                    "null_object_pattern",
                    format!(
                        "Field `{}` is null-checked in {} functions — consider Null Object pattern",
                        field, count
                    ),
                ));
            }
        }
    }

    fn check_template_method(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for c in &ctx.model.classes {
            if c.self_call_count >= self.template_min_self_calls
                && c.method_count >= self.template_min_methods
            {
                findings.push(hint(
                    ctx,
                    (c.start_line, c.name_col, c.name_end_col, Some(&c.name)),
                    "template_method_pattern",
                    format!(
                        "Class `{}` has a method calling {} self-methods — consider Template Method",
                        c.name, c.self_call_count
                    ),
                ));
            }
        }
    }
}

/// Observer: class has listener fields and/or notify methods.
fn check_observer(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    for c in &ctx.model.classes {
        let msg = match (c.has_listener_field, c.has_notify_method) {
            (true, true) => format!(
                "Class `{}` uses Observer pattern — ensure proper subscribe/unsubscribe lifecycle",
                c.name
            ),
            (true, false) => format!(
                "Class `{}` has listener fields but no notify method — consider completing Observer",
                c.name
            ),
            _ => continue,
        };
        findings.push(hint(
            ctx,
            (c.start_line, c.name_col, c.name_end_col, Some(&c.name)),
            "observer_pattern",
            msg,
        ));
    }
}

fn hint(
    ctx: &AnalysisContext,
    loc: (usize, usize, usize, Option<&str>),
    smell: &str,
    message: String,
) -> Finding {
    Finding {
        smell_name: smell.into(),
        category: SmellCategory::OoAbusers,
        severity: Severity::Hint,
        location: Location {
            path: ctx.file.path.clone(),
            start_line: loc.0,
            start_col: loc.1,
            end_line: loc.0,
            end_col: loc.2,
            name: loc.3.map(String::from),
        },
        message,
        suggested_refactorings: vec![],
        ..Default::default()
    }
}
