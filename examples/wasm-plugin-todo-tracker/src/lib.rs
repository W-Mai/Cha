//! Enhanced TODO comment tracker.
//!
//! Goes beyond the builtin `todo_tracker` (which fixed-set HACK/XXX/FIXME/TODO
//! with hardcoded severities) by adding:
//!
//! 1. `extended_todo_tag` — extra tags (BUG/WIP/OPTIMIZE/PERF/DEPRECATED) +
//!    user-configurable extras. Parses metadata: `(@author)`, `(#issue)`,
//!    `(by:YYYY-MM-DD)` for expiration.
//! 2. `expired_todo` — comment with `(by:YYYY-MM-DD)` past today's date.
//! 3. `priority_todo` — `!` / `!!` / `!!!` after the tag escalates severity.
//!    A triple-bang `FIXME!!!` becomes Error.
//! 4. `todo_hotspot` — file has > N todos per 100 LOC, or > M absolute todos.
//!    One finding per file.
//! 5. `attributed_todo_missing` — opt-in: when `require_attribution = true`
//!    in `.cha.toml`, todos without `(@author)` or `(#issue)` are flagged.

cha_plugin_sdk::plugin!(TodoTrackerPlugin);

struct TodoTrackerPlugin;

/// Tags this plugin defines beyond the builtin set. Each gets an
/// `extended_todo_tag` finding plus shared metadata (priority, expiration).
const DEFAULT_TAGS: &[(&str, Severity)] = &[
    ("BUG", Severity::Warning),
    ("WIP", Severity::Hint),
    ("OPTIMIZE", Severity::Hint),
    ("PERF", Severity::Hint),
    ("DEPRECATED", Severity::Hint),
];

/// Tags handled by the builtin `todo_tracker`. We still parse them so
/// priority/expiration findings can fire on `FIXME!!!` etc., but we don't
/// emit `extended_todo_tag` for them (that's the builtin's job).
const BUILTIN_TAGS: &[&str] = &["TODO", "FIXME", "HACK", "XXX"];

/// Default density threshold: at most N todos per 100 lines of code.
const DEFAULT_DENSITY_PER_100: f64 = 10.0;
/// Default absolute count threshold per file.
const DEFAULT_COUNT_THRESHOLD: usize = 20;

impl PluginImpl for TodoTrackerPlugin {
    fn name() -> String {
        "todo-tracker".into()
    }

    fn smells() -> Vec<String> {
        vec![
            "extended_todo_tag".into(),
            "expired_todo".into(),
            "priority_todo".into(),
            "todo_hotspot".into(),
            "attributed_todo_missing".into(),
        ]
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        if input.comments.is_empty() {
            return vec![];
        }

        let cfg = read_config(&input.options);
        let mut findings = Vec::new();

        for c in &input.comments {
            scan_comment(&input, c, &cfg, &mut findings);
        }

        check_hotspot(&input, &cfg, &mut findings);
        findings
    }
}

// === Configuration ===

struct Config {
    extra_tags: Vec<(String, Severity)>,
    require_attribution: bool,
    today: Option<String>, // YYYY-MM-DD; if None, expiration check is skipped
    density_per_100: f64,
    count_threshold: usize,
}

fn read_config(options: &[(String, OptionValue)]) -> Config {
    let extra_tags_list = cha_plugin_sdk::option_list_str!(options, "extra_tags")
        .map(|l| l.to_vec())
        .unwrap_or_default();
    let extra_tags_warning = cha_plugin_sdk::option_list_str!(options, "extra_tags_warning")
        .map(|l| l.to_vec())
        .unwrap_or_default();
    let warning_set: std::collections::HashSet<&str> =
        extra_tags_warning.iter().map(|s| s.as_str()).collect();
    let extra_tags = extra_tags_list
        .into_iter()
        .map(|t| {
            let sev = if warning_set.contains(t.as_str()) {
                Severity::Warning
            } else {
                Severity::Hint
            };
            (t.to_uppercase(), sev)
        })
        .collect();

    let require_attribution =
        cha_plugin_sdk::option_bool!(options, "require_attribution").unwrap_or(false);
    let today = cha_plugin_sdk::option_str!(options, "today").map(String::from);
    let density_per_100 = cha_plugin_sdk::option_float!(options, "density_per_100")
        .unwrap_or(DEFAULT_DENSITY_PER_100);
    let count_threshold = cha_plugin_sdk::option_int!(options, "count_threshold")
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_COUNT_THRESHOLD);

    Config {
        extra_tags,
        require_attribution,
        today,
        density_per_100,
        count_threshold,
    }
}

// === Comment scanning ===

#[derive(Default)]
struct ParsedComment<'a> {
    tag: Option<&'a str>,
    severity: Option<Severity>,
    /// True if `tag` is one of TODO/FIXME/HACK/XXX (builtin handles these).
    /// We still parse priority/metadata for them but suppress
    /// `extended_todo_tag` to avoid duplication.
    is_builtin_tag: bool,
    priority: u8, // 0 = none, 1 = !, 2 = !!, 3 = !!!
    author: Option<String>,
    issue: Option<String>,
    expires: Option<String>,
}

fn scan_comment(
    input: &AnalysisInput,
    c: &CommentInfo,
    cfg: &Config,
    findings: &mut Vec<Finding>,
) {
    let parsed = parse_comment(&c.text, cfg);
    let Some(tag) = parsed.tag else {
        return;
    };

    let sev = effective_severity(&parsed);
    let metadata = format_metadata(&parsed);

    // Smell #1: extended tag — only for tags this plugin owns
    // (BUILTIN_TAGS are reported by the builtin todo_tracker).
    if !parsed.is_builtin_tag {
        findings.push(Finding {
            smell_name: "extended_todo_tag".into(),
            category: SmellCategory::Dispensables,
            severity: sev,
            location: build_loc(input, c.line, tag),
            message: format!(
                "{} comment found{}: {}",
                tag,
                metadata,
                short_text(&c.text)
            ),
            suggested_refactorings: vec!["Resolve or convert to tracked issue".into()],
            actual_value: None,
            threshold: None,
        });
    }

    // Smell #2: expired
    if let Some(date) = &parsed.expires
        && is_expired(date, cfg.today.as_deref())
    {
        findings.push(Finding {
            smell_name: "expired_todo".into(),
            category: SmellCategory::Dispensables,
            severity: Severity::Warning,
            location: build_loc(input, c.line, tag),
            message: format!("{} expired on {} — past due", tag, date),
            suggested_refactorings: vec!["Resolve or extend the expiration".into()],
            actual_value: None,
            threshold: None,
        });
    }

    // Smell #3: priority bump as a separate finding (so users can disable it
    // independently from extended_todo_tag)
    if parsed.priority > 0 {
        let bang_severity = match parsed.priority {
            1 => Severity::Hint,
            2 => Severity::Warning,
            _ => Severity::Error, // 3+
        };
        findings.push(Finding {
            smell_name: "priority_todo".into(),
            category: SmellCategory::Dispensables,
            severity: bang_severity,
            location: build_loc(input, c.line, tag),
            message: format!(
                "{} marked priority {} — escalated severity",
                tag,
                "!".repeat(parsed.priority as usize)
            ),
            suggested_refactorings: vec!["Address before merging".into()],
            actual_value: Some(parsed.priority as f64),
            threshold: Some(1.0),
        });
    }

    // Smell #5: missing attribution
    if cfg.require_attribution && parsed.author.is_none() && parsed.issue.is_none() {
        findings.push(Finding {
            smell_name: "attributed_todo_missing".into(),
            category: SmellCategory::Dispensables,
            severity: Severity::Hint,
            location: build_loc(input, c.line, tag),
            message: format!(
                "{} has no `(@author)` or `(#issue)` attribution",
                tag
            ),
            suggested_refactorings: vec![
                "Add `(@yourname)` or link to a tracked issue `(#NNN)`".into(),
            ],
            actual_value: None,
            threshold: None,
        });
    }
}

fn parse_comment<'a>(text: &'a str, cfg: &'a Config) -> ParsedComment<'a> {
    let mut out = ParsedComment::default();

    // Find the leading tag. Order: extra_tags → DEFAULT_TAGS → BUILTIN_TAGS.
    // Builtin tags are recognized so priority/expiration can fire, but
    // `is_builtin_tag` flags them to suppress the duplicate `extended_todo_tag`.
    let upper = text.to_uppercase();
    let candidates = cfg
        .extra_tags
        .iter()
        .map(|(t, s)| (t.as_str(), *s, false))
        .chain(DEFAULT_TAGS.iter().map(|(t, s)| (*t, *s, false)))
        .chain(BUILTIN_TAGS.iter().map(|t| (*t, Severity::Hint, true)));
    for (tag, sev, is_builtin) in candidates {
        if let Some(pos) = find_tag_at_word_boundary(&upper, tag) {
            let original_tag = &text[pos..pos + tag.len()];
            out.tag = Some(original_tag);
            out.severity = Some(sev);
            out.is_builtin_tag = is_builtin;
            let mut p = 0u8;
            for ch in text[pos + tag.len()..].chars().take(4) {
                if ch == '!' {
                    p = p.saturating_add(1);
                } else {
                    break;
                }
            }
            out.priority = p;
            break;
        }
    }

    out.author = parse_paren_field(text, "@");
    out.issue = parse_paren_field(text, "#");
    out.expires = parse_expires_field(text);

    out
}

/// Find `(prefix<value>)` where prefix is `@` or `#` and value is `[A-Za-z0-9_-]+`.
fn parse_paren_field(text: &str, prefix: &str) -> Option<String> {
    let needle = format!("({}", prefix);
    let start = text.find(&needle)?;
    let after = &text[start + needle.len()..];
    let end = after.find(|c: char| !is_ident_or_dash(c))?;
    if end == 0 {
        return None;
    }
    let value = &after[..end];
    // Must close with ')'
    if after.as_bytes().get(end) != Some(&b')') {
        return None;
    }
    Some(value.to_string())
}

/// Find `(by:YYYY-MM-DD)` or `(expires:YYYY-MM-DD)`.
fn parse_expires_field(text: &str) -> Option<String> {
    for prefix in &["(by:", "(expires:", "(expire:"] {
        if let Some(start) = text.find(prefix) {
            let after = &text[start + prefix.len()..];
            // Read up to next `)`; must be exactly YYYY-MM-DD format.
            let end = after.find(')')?;
            let date = &after[..end];
            if is_ymd_format(date) {
                return Some(date.to_string());
            }
        }
    }
    None
}

fn is_ident_or_dash(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn is_ymd_format(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    bytes
        .iter()
        .enumerate()
        .all(|(i, b)| if i == 4 || i == 7 { *b == b'-' } else { b.is_ascii_digit() })
}

fn find_tag_at_word_boundary(upper_text: &str, tag: &str) -> Option<usize> {
    let bytes = upper_text.as_bytes();
    let tag_bytes = tag.as_bytes();
    let mut i = 0;
    while let Some(pos) = upper_text[i..].find(tag) {
        let abs = i + pos;
        let before_ok =
            abs == 0 || !is_ident_char(bytes.get(abs.wrapping_sub(1)).copied().unwrap_or(b' '));
        let after_ok = bytes
            .get(abs + tag_bytes.len())
            .map(|b| !is_ident_char(*b))
            .unwrap_or(true);
        if before_ok && after_ok {
            return Some(abs);
        }
        i = abs + tag_bytes.len();
    }
    None
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn effective_severity(parsed: &ParsedComment) -> Severity {
    let base = parsed.severity.unwrap_or(Severity::Hint);
    match parsed.priority {
        0 => base,
        1 => Severity::Hint,
        2 => Severity::Warning,
        _ => Severity::Error,
    }
}

fn format_metadata(parsed: &ParsedComment) -> String {
    let mut parts = Vec::new();
    if let Some(a) = &parsed.author {
        parts.push(format!("@{}", a));
    }
    if let Some(i) = &parsed.issue {
        parts.push(format!("#{}", i));
    }
    if let Some(d) = &parsed.expires {
        parts.push(format!("by:{}", d));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(" "))
    }
}

fn short_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() > 80 {
        format!("{}…", &trimmed[..77])
    } else {
        trimmed.to_string()
    }
}

fn build_loc(input: &AnalysisInput, line: u32, _tag: &str) -> Location {
    Location {
        path: input.path.clone(),
        start_line: line,
        start_col: 0,
        end_line: line,
        end_col: 0,
        name: None,
    }
}

// === Date / expiration ===

fn is_expired(date: &str, today_override: Option<&str>) -> bool {
    let Some(today) = today_override else {
        // Without an explicit `today` option, don't compare against the
        // wall clock — WASI clock support varies and we'd rather miss
        // expired-todo detection than panic.
        return false;
    };
    // Strings sort lexicographically as dates because YYYY-MM-DD is fixed-width.
    date < today
}

// === Hotspot ===

fn check_hotspot(input: &AnalysisInput, cfg: &Config, findings: &mut Vec<Finding>) {
    let count = count_total_todos(input);
    if count == 0 || input.total_lines == 0 {
        return;
    }
    let density = (count as f64) * 100.0 / input.total_lines as f64;
    if count > cfg.count_threshold || density > cfg.density_per_100 {
        findings.push(Finding {
            smell_name: "todo_hotspot".into(),
            category: SmellCategory::Dispensables,
            severity: Severity::Warning,
            location: Location {
                path: input.path.clone(),
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 0,
                name: None,
            },
            message: format!(
                "TODO hotspot: {} todos in {} lines ({:.1} per 100 LOC)",
                count, input.total_lines, density
            ),
            suggested_refactorings: vec!["Schedule a cleanup pass for this file".into()],
            actual_value: Some(density),
            threshold: Some(cfg.density_per_100),
        });
    }
}

fn count_total_todos(input: &AnalysisInput) -> usize {
    // Cheap: count comments mentioning any tag (default + extra).
    // Builtin tags too — hotspot density should reflect *all* todo-like
    // comments regardless of which plugin reports each one.
    let mut n = 0;
    for c in &input.comments {
        let upper = c.text.to_uppercase();
        for tag in ["TODO", "FIXME", "HACK", "XXX", "BUG", "WIP", "OPTIMIZE", "PERF", "DEPRECATED"]
        {
            if find_tag_at_word_boundary(&upper, tag).is_some() {
                n += 1;
                break;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use cha_plugin_sdk::test_utils::WasmPluginTest;

    #[test]
    fn detects_bug_tag() {
        WasmPluginTest::new()
            .source("typescript", "// BUG: this fails on weekends\nconsole.log('hi');")
            .assert_finding("extended_todo_tag");
    }

    #[test]
    fn detects_wip_tag() {
        WasmPluginTest::new()
            .source("typescript", "// WIP: still working on this")
            .assert_finding("extended_todo_tag");
    }

    #[test]
    fn does_not_flag_plain_todo_as_extended() {
        // Plain TODO is the builtin's job; this plugin only adds the extra tags.
        WasmPluginTest::new()
            .source("typescript", "// TODO: clean this up")
            .assert_no_finding_named("extended_todo_tag");
    }

    #[test]
    fn detects_priority_todo() {
        WasmPluginTest::new()
            .source("typescript", "// BUG!!! must fix before release")
            .assert_finding("priority_todo");
    }

    #[test]
    fn ignores_word_containing_tag() {
        // "BUGGY" should not match "BUG" because of word-boundary check.
        WasmPluginTest::new()
            .source("typescript", "// BUGGY behavior here")
            .assert_no_finding_named("extended_todo_tag");
    }

    #[test]
    fn detects_expired_todo_with_explicit_today() {
        WasmPluginTest::new()
            .source("typescript", "// BUG: (by:2020-01-01) old issue")
            .option("today", "2025-05-21")
            .assert_finding("expired_todo");
    }

    #[test]
    fn no_expired_when_in_future() {
        WasmPluginTest::new()
            .source("typescript", "// BUG: (by:2099-12-31) future")
            .option("today", "2025-05-21")
            .assert_no_finding_named("expired_todo");
    }

    // Note: extra_tags via options is a list-of-string, but the current
    // test_utils only supports string options. Manual e2e test with .cha.toml
    // covers list-option config.
}
