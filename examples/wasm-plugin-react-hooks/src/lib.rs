//! React hooks rule violations detector.
//!
//! Implements 5 detections from the canonical "rules of hooks":
//!
//! 1. `hook_called_from_non_component` — calling a hook from a function that
//!    is neither a React component (capitalized name) nor a custom hook
//!    (`use[A-Z_]` name). Pure data scan, no tree_query.
//! 2. `conditional_hook_call` — hook call inside `if` / `?:` / `&&`/`||` / `try`.
//!    Uses tree_query: collect all hook-call ranges and all guard ranges,
//!    intersect by line/column position.
//! 3. `hook_in_loop` — hook call inside `for` / `while` / `do-while`.
//! 4. `hook_after_early_return` — hook call appearing after a `return` /
//!    `throw` within the same function body.
//! 5. `nested_function_hook_call` — hook call inside a nested arrow /
//!    function expression that is not the analyzed function itself.
//!
//! Runs only on TypeScript / TSX files (`input.language == "typescript"`).
//! Skips test files (`input.role == FileRole::Test`).
//!
//! Hook identification by naming convention only (no type info available
//! statically): a hook is any function whose name matches `^use[A-Z_]`.

cha_plugin_sdk::plugin!(ReactHooksPlugin);

struct ReactHooksPlugin;

impl PluginImpl for ReactHooksPlugin {
    fn name() -> String {
        "react-hooks".into()
    }

    fn smells() -> Vec<String> {
        vec![
            "hook_called_from_non_component".into(),
            "conditional_hook_call".into(),
            "hook_in_loop".into(),
            "hook_after_early_return".into(),
            "nested_function_hook_call".into(),
        ]
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        if input.language != "typescript" {
            return vec![];
        }
        if input.role == FileRole::Test {
            return vec![];
        }

        let mut findings = Vec::new();
        check_hook_from_non_component(&input, &mut findings);
        check_positional_hook_violations(&input, &mut findings);
        findings
    }
}

// === Naming heuristics ===

/// True if `name` matches the React hook convention `^use[A-Z_]`.
/// Handles `useState` / `useMyThing` / `use_internal`, rejects
/// `username` / `useragent` / `useless`. Also strips an optional
/// member-access prefix so `React.useState` is recognized.
fn is_hook_name(name: &str) -> bool {
    let bare = name.rsplit('.').next().unwrap_or(name);
    if !bare.starts_with("use") {
        return false;
    }
    bare.chars()
        .nth(3)
        .is_some_and(|c| c.is_uppercase() || c == '_')
}

/// True if `name` looks like a React component (capitalized first letter).
fn is_component_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

// === Smell #1: hook called from non-component / non-hook ===

fn check_hook_from_non_component(input: &AnalysisInput, findings: &mut Vec<Finding>) {
    for f in &input.functions {
        if is_component_name(&f.name) || is_hook_name(&f.name) {
            continue;
        }
        let mut hooks_called: Vec<&str> = f
            .called_functions
            .iter()
            .filter(|c| is_hook_name(c))
            .map(|s| s.as_str())
            .collect();
        if hooks_called.is_empty() {
            continue;
        }
        hooks_called.sort();
        hooks_called.dedup();
        findings.push(Finding {
            smell_name: "hook_called_from_non_component".into(),
            category: SmellCategory::OoAbusers,
            severity: Severity::Warning,
            location: Location {
                path: input.path.clone(),
                start_line: f.start_line,
                start_col: f.name_col,
                end_line: f.start_line,
                end_col: f.name_end_col,
                name: Some(f.name.clone()),
            },
            message: format!(
                "Function `{}` is neither a component nor a hook, but calls hook(s): {}",
                f.name,
                hooks_called.join(", ")
            ),
            suggested_refactorings: vec![
                "Rename to `use{Name}` if this should be a custom hook".into(),
                "Or move the hook call to a component/hook caller".into(),
            ],
            actual_value: None,
            threshold: None,
        });
    }
}

// === Smells #2-5: positional violations using tree_query ===

#[derive(Clone)]
struct HookCallSite {
    line: u32,
    col: u32,
    end_line: u32,
    end_col: u32,
    name: String,
}

#[derive(Clone)]
struct Range {
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
}

impl Range {
    fn from_match(m: &tree_query::QueryMatch) -> Self {
        Self {
            start_line: m.start_line,
            start_col: m.start_col,
            end_line: m.end_line,
            end_col: m.end_col,
        }
    }

    fn contains_point(&self, line: u32, col: u32) -> bool {
        let after_start = line > self.start_line
            || (line == self.start_line && col >= self.start_col);
        let before_end = line < self.end_line
            || (line == self.end_line && col <= self.end_col);
        after_start && before_end
    }

    fn contains_call(&self, c: &HookCallSite) -> bool {
        // The call sits inside this range if its start position is inside.
        self.contains_point(c.line, c.col)
    }
}

fn check_positional_hook_violations(input: &AnalysisInput, findings: &mut Vec<Finding>) {
    let queries = vec![
        // 0: hook calls
        "(call_expression function: (identifier) @h (#match? @h \"^use[A-Z_]\")) @hook_call".into(),
        // 1: conditional guards
        "(if_statement) @g".into(),
        "(ternary_expression) @g".into(),
        "(try_statement) @g".into(),
        "(binary_expression) @g".into(),
        // 5: loop guards
        "(for_statement) @g".into(),
        "(while_statement) @g".into(),
        "(do_statement) @g".into(),
        "(for_in_statement) @g".into(),
        // 9: early-return statements
        "(return_statement) @r".into(),
        "(throw_statement) @r".into(),
        // 11: nested function expressions
        "(arrow_function) @nf".into(),
        "(function_expression) @nf".into(),
    ];

    let results = tree_query::run_queries(&queries);
    if results.len() < queries.len() {
        return;
    }

    let hook_calls = collect_hook_calls(&results[0]);
    if hook_calls.is_empty() {
        return;
    }

    let cond_guards = collect_ranges(&results[1..=4]);
    let loop_guards = collect_ranges(&results[5..=8]);
    let returns = collect_ranges(&results[9..=10]);
    let nested_fns = collect_ranges(&results[11..=12]);

    for hook in &hook_calls {
        if let Some(g) = find_innermost(&cond_guards, hook) {
            findings.push(make_positional_finding(
                input,
                hook,
                "conditional_hook_call",
                "inside a conditional branch",
                g.start_line,
            ));
        }
        if let Some(g) = find_innermost(&loop_guards, hook) {
            findings.push(make_positional_finding(
                input,
                hook,
                "hook_in_loop",
                "inside a loop body",
                g.start_line,
            ));
        }
        if is_after_return(input, hook, &returns) {
            findings.push(make_positional_finding(
                input,
                hook,
                "hook_after_early_return",
                "after an earlier `return` or `throw`",
                hook.line,
            ));
        }
        if is_in_nested_fn(input, hook, &nested_fns) {
            findings.push(make_positional_finding(
                input,
                hook,
                "nested_function_hook_call",
                "inside a nested function / callback",
                hook.line,
            ));
        }
    }
}

fn collect_hook_calls(matches: &[Vec<tree_query::QueryMatch>]) -> Vec<HookCallSite> {
    let mut out = Vec::new();
    for m in matches {
        let call_match = m.iter().find(|c| c.capture_name == "hook_call");
        let name_match = m.iter().find(|c| c.capture_name == "h");
        if let (Some(call), Some(name)) = (call_match, name_match) {
            out.push(HookCallSite {
                line: call.start_line,
                col: call.start_col,
                end_line: call.end_line,
                end_col: call.end_col,
                name: name.text.clone(),
            });
        }
    }
    out
}

fn collect_ranges(query_results: &[Vec<Vec<tree_query::QueryMatch>>]) -> Vec<Range> {
    let mut out = Vec::new();
    for matches in query_results {
        for m in matches {
            for cap in m {
                out.push(Range::from_match(cap));
            }
        }
    }
    out
}

fn find_innermost(ranges: &[Range], hook: &HookCallSite) -> Option<Range> {
    ranges
        .iter()
        .filter(|r| r.contains_call(hook))
        .min_by_key(|r| (r.end_line - r.start_line, r.end_col.saturating_sub(r.start_col)))
        .cloned()
}

fn is_after_return(input: &AnalysisInput, hook: &HookCallSite, returns: &[Range]) -> bool {
    // Resolve the declared function this hook lives in via the host —
    // function_at handles nested + multi-function disambiguation that
    // input.functions alone can't.
    let Some(host) = project_query::function_at(&input.path, hook.line, hook.col) else {
        return false;
    };
    let host_start = host.start_line;
    let host_end = host.end_line;
    returns.iter().any(|r| {
        // Return must be inside the host function (1-based)...
        r.start_line >= host_start
            && r.end_line <= host_end
            // ...and the entire return statement must end BEFORE the hook
            // starts. Otherwise the hook is inside the return expression
            // itself (e.g. `return useState(0)` or `return <div>{useState()}</div>`),
            // which is not an early-return violation.
            && (r.end_line, r.end_col) < (hook.line, hook.col)
    })
}

fn is_in_nested_fn(input: &AnalysisInput, hook: &HookCallSite, nested: &[Range]) -> bool {
    // A hook is "nested" if its call sits inside an arrow / function-expression
    // strictly contained in the host function — i.e. a callback, not the
    // host's own body.
    let Some(host) = project_query::function_at(&input.path, hook.line, hook.col) else {
        return false;
    };
    for nf in nested {
        if !nf.contains_call(hook) {
            continue;
        }
        // Strictly inside host: starts after host start OR ends before host end.
        if nf.start_line > host.start_line || nf.end_line < host.end_line {
            return true;
        }
    }
    false
}

fn make_positional_finding(
    input: &AnalysisInput,
    hook: &HookCallSite,
    smell: &str,
    where_msg: &str,
    _hint_line: u32,
) -> Finding {
    Finding {
        smell_name: smell.into(),
        category: SmellCategory::OoAbusers,
        severity: Severity::Warning,
        location: Location {
            path: input.path.clone(),
            start_line: hook.line,
            start_col: hook.col,
            end_line: hook.end_line,
            end_col: hook.end_col,
            name: Some(hook.name.clone()),
        },
        message: format!(
            "Hook `{}` is called {} — violates Rules of Hooks",
            hook.name, where_msg
        ),
        suggested_refactorings: vec![
            "Move the hook call to the top level of the component/hook body".into(),
        ],
        actual_value: None,
        threshold: None,
    }
}

#[cfg(test)]
mod tests {
    use cha_plugin_sdk::test_utils::WasmPluginTest;

    #[test]
    fn detects_hook_called_from_non_component() {
        WasmPluginTest::new()
            .source(
                "typescript",
                r#"function plainHelper() {
    const [count, setCount] = useState(0);
    return count;
}"#,
            )
            .assert_finding("hook_called_from_non_component");
    }

    #[test]
    fn does_not_flag_hook_in_component() {
        WasmPluginTest::new()
            .source(
                "typescript",
                r#"function MyComponent() {
    const [count, setCount] = useState(0);
    return count;
}"#,
            )
            .assert_no_finding_named("hook_called_from_non_component");
    }

    #[test]
    fn does_not_flag_hook_in_custom_hook() {
        WasmPluginTest::new()
            .source(
                "typescript",
                r#"function useMyData() {
    const [data, setData] = useState(null);
    return data;
}"#,
            )
            .assert_no_finding_named("hook_called_from_non_component");
    }

    #[test]
    fn naming_heuristic_rejects_useless_username() {
        // Functions whose name starts with "use" but second char isn't [A-Z_]
        // are not hooks (e.g. `username`, `useragent`).
        WasmPluginTest::new()
            .source(
                "typescript",
                r#"function username() { return 1; }
function User() {
    const u = username();
    return u;
}"#,
            )
            .assert_no_finding();
    }
}
