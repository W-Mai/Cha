use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Suggest design patterns based on AST structural signals.
pub struct DesignPatternAdvisor;

impl Default for DesignPatternAdvisor {
    fn default() -> Self {
        Self
    }
}

impl Plugin for DesignPatternAdvisor {
    fn name(&self) -> &str {
        "design_pattern"
    }

    fn description(&self) -> &str {
        "Suggest design patterns based on code structure"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        check_strategy(ctx, &mut findings);
        check_state(ctx, &mut findings);
        check_builder(ctx, &mut findings);
        check_null_object(ctx, &mut findings);
        check_template_method(ctx, &mut findings);
        check_observer(ctx, &mut findings);
        findings
    }
}

/// Strategy: function dispatches on a type/kind field with many arms.
fn check_strategy(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    for f in &ctx.model.functions {
        let target = match f.switch_dispatch_target.as_deref() {
            Some(t) if f.switch_arms >= 4 && is_type_field(t) => t,
            _ => continue,
        };
        findings.push(hint(
            ctx,
            (f.start_line, f.end_line, Some(&f.name)),
            "strategy_pattern",
            format!(
                "Function `{}` dispatches on `{}` with {} arms — consider Strategy pattern",
                f.name, target, f.switch_arms
            ),
        ));
    }
}

/// State: switch/match on a state/status field.
fn check_state(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    for f in &ctx.model.functions {
        let target = match f.switch_dispatch_target.as_deref() {
            Some(t) if f.switch_arms >= 3 && is_state_field(t) => t,
            _ => continue,
        };
        findings.push(hint(
            ctx,
            (f.start_line, f.end_line, Some(&f.name)),
            "state_pattern",
            format!(
                "Function `{}` dispatches on `{}` — consider State pattern",
                f.name, target
            ),
        ));
    }
}

/// Builder: function with many parameters, especially optional ones.
fn check_builder(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    for f in &ctx.model.functions {
        if f.parameter_count >= 7 || (f.parameter_count >= 5 && f.optional_param_count >= 3) {
            findings.push(hint(
                ctx,
                (f.start_line, f.end_line, Some(&f.name)),
                "builder_pattern",
                format!(
                    "Function `{}` has {} params ({} optional) — consider Builder pattern",
                    f.name, f.parameter_count, f.optional_param_count
                ),
            ));
        }
    }
}

/// Null Object: repeated null checks on the same field across functions.
fn check_null_object(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for f in &ctx.model.functions {
        for field in &f.null_check_fields {
            *counts.entry(field).or_default() += 1;
        }
    }
    for (field, count) in &counts {
        if *count >= 3 {
            findings.push(hint(
                ctx,
                (1, ctx.model.total_lines, None),
                "null_object_pattern",
                format!(
                    "Field `{}` is null-checked in {} functions — consider Null Object pattern",
                    field, count
                ),
            ));
        }
    }
}

/// Template Method: class has a method calling many self-methods.
fn check_template_method(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    for c in &ctx.model.classes {
        if c.self_call_count >= 3 && c.method_count >= 4 {
            findings.push(hint(
                ctx,
                (c.start_line, c.end_line, Some(&c.name)),
                "template_method_pattern",
                format!(
                    "Class `{}` has a method calling {} self-methods — consider Template Method",
                    c.name, c.self_call_count
                ),
            ));
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
            (c.start_line, c.end_line, Some(&c.name)),
            "observer_pattern",
            msg,
        ));
    }
}

fn is_type_field(name: &str) -> bool {
    let l = name.to_lowercase();
    l.contains("type")
        || l.contains("kind")
        || l.contains("role")
        || l.contains("action")
        || l.contains("mode")
}

fn is_state_field(name: &str) -> bool {
    let l = name.to_lowercase();
    l.contains("state") || l.contains("status")
}

fn hint(
    ctx: &AnalysisContext,
    loc: (usize, usize, Option<&str>),
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
            end_line: loc.1,
            name: loc.2.map(String::from),
        },
        message,
        suggested_refactorings: vec![],
    }
}
