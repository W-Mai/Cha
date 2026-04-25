use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory, TypeRef};

/// Known async / channel handle type names across Rust, TypeScript, Python,
/// Go. Matched on the innermost type identifier (via TypeRef::name), so
/// `tokio::task::JoinHandle<T>` and `JoinHandle` both match.
const ASYNC_HANDLE_TYPES: &[&str] = &[
    // Rust async ecosystem
    "JoinHandle",
    "Future",
    "Task",
    "AbortHandle",
    "oneshot",
    "mpsc",
    // Channel halves (cross-ecosystem)
    "Sender",
    "Receiver",
    "UnboundedSender",
    "UnboundedReceiver",
    "WatchSender",
    "WatchReceiver",
    // JavaScript / TypeScript
    "Promise",
    "PromiseLike",
    // Go (channel types are punctuation so they surface as Unknown — leave to
    // a later pass; we still catch `context.CancelFunc`, `sync.WaitGroup`).
    "CancelFunc",
    "WaitGroup",
    // Python
    "Awaitable",
    "Coroutine",
    "Queue",
];

/// Names that legitimately return async handles — launchers/spawners whose
/// whole point is to expose a handle. Skip them to keep the signal tight.
const LAUNCHER_PREFIXES: &[&str] = &[
    "spawn",
    "spawn_",
    "launch",
    "launch_",
    "start",
    "start_",
    "run_async",
    "fire_",
    "dispatch_",
    "background_",
];

#[derive(Default)]
pub struct AsyncCallbackLeakAnalyzer;

impl Plugin for AsyncCallbackLeakAnalyzer {
    fn name(&self) -> &str {
        "async_callback_leak"
    }

    fn smells(&self) -> Vec<String> {
        vec!["async_callback_leak".into()]
    }

    fn description(&self) -> &str {
        "Function signature leaks a raw async handle (JoinHandle/Future/Channel)"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter_map(|f| {
                if is_launcher_shaped(&f.name) {
                    return None;
                }
                if let Some(ret) = &f.return_type
                    && is_async_handle(ret)
                {
                    return Some(build_finding(ctx, f, ret, Position::Return));
                }
                for (idx, param) in f.parameter_types.iter().enumerate() {
                    if is_async_handle(param) {
                        return Some(build_finding(ctx, f, param, Position::Param(idx + 1)));
                    }
                }
                None
            })
            .collect()
    }
}

fn is_launcher_shaped(name: &str) -> bool {
    LAUNCHER_PREFIXES
        .iter()
        .any(|p| name == *p || name.starts_with(p))
}

fn is_async_handle(t: &TypeRef) -> bool {
    ASYNC_HANDLE_TYPES.contains(&t.name.as_str())
}

enum Position {
    Return,
    Param(usize),
}

fn build_finding(
    ctx: &AnalysisContext,
    f: &crate::FunctionInfo,
    t: &TypeRef,
    pos: Position,
) -> Finding {
    let (where_it, suggestion) = match pos {
        Position::Return => (
            "return type".to_string(),
            "Wait inside the function and return a domain value, or wrap the handle in a local Task abstraction".to_string(),
        ),
        Position::Param(i) => (
            format!("parameter #{i}"),
            "Accept a domain callback/value instead of a raw async handle".to_string(),
        ),
    };
    Finding {
        smell_name: "async_callback_leak".into(),
        category: SmellCategory::Couplers,
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
            "Function `{}` has `{}` in its {} — concurrency primitive leaks to callers",
            f.name, t.name, where_it
        ),
        suggested_refactorings: vec![
            suggestion,
            "Expose a higher-level interface (domain event, callback) instead of the raw handle"
                .into(),
        ],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionInfo, SourceFile, SourceModel, TypeOrigin};
    use std::path::PathBuf;

    fn ctx_with(functions: Vec<FunctionInfo>) -> (SourceFile, SourceModel) {
        let file = SourceFile::new(PathBuf::from("test.rs"), String::new());
        let model = SourceModel {
            language: "rust".into(),
            total_lines: 10,
            functions,
            classes: vec![],
            imports: vec![],
            comments: vec![],
            type_aliases: vec![],
        };
        (file, model)
    }

    fn tref(name: &str, origin: TypeOrigin) -> TypeRef {
        TypeRef {
            name: name.into(),
            raw: name.into(),
            origin,
        }
    }

    fn run(functions: Vec<FunctionInfo>) -> Vec<Finding> {
        let (file, model) = ctx_with(functions);
        let ctx = AnalysisContext {
            file: &file,
            model: &model,
        };
        AsyncCallbackLeakAnalyzer.analyze(&ctx)
    }

    #[test]
    fn flags_function_returning_join_handle() {
        let f = FunctionInfo {
            name: "load_user".into(),
            start_line: 1,
            end_line: 5,
            return_type: Some(tref("JoinHandle", TypeOrigin::External("tokio".into()))),
            ..Default::default()
        };
        let findings = run(vec![f]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("JoinHandle"));
        assert!(findings[0].message.contains("return type"));
    }

    #[test]
    fn flags_function_taking_sender() {
        let f = FunctionInfo {
            name: "configure".into(),
            start_line: 1,
            end_line: 5,
            parameter_count: 1,
            parameter_types: vec![tref("Sender", TypeOrigin::External("tokio".into()))],
            ..Default::default()
        };
        let findings = run(vec![f]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Sender"));
        assert!(findings[0].message.contains("parameter #1"));
    }

    #[test]
    fn ignores_launcher_shaped_names() {
        // spawn_worker returning JoinHandle is legitimate — that's the whole
        // point of a spawn function.
        let f = FunctionInfo {
            name: "spawn_worker".into(),
            start_line: 1,
            end_line: 5,
            return_type: Some(tref("JoinHandle", TypeOrigin::External("tokio".into()))),
            ..Default::default()
        };
        let findings = run(vec![f]);
        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_plain_domain_signatures() {
        let f = FunctionInfo {
            name: "get_user".into(),
            start_line: 1,
            end_line: 5,
            return_type: Some(tref("User", TypeOrigin::Local)),
            parameter_count: 1,
            parameter_types: vec![tref("UserId", TypeOrigin::Local)],
            ..Default::default()
        };
        let findings = run(vec![f]);
        assert!(findings.is_empty());
    }

    #[test]
    fn flags_promise_typescript() {
        let f = FunctionInfo {
            name: "fetch_users".into(),
            start_line: 1,
            end_line: 5,
            return_type: Some(tref("Promise", TypeOrigin::Primitive)),
            ..Default::default()
        };
        let findings = run(vec![f]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Promise"));
    }
}
