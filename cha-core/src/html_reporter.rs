use crate::health::HealthScore;
use crate::{Finding, Severity};
use std::collections::BTreeMap;
use std::fmt::Write;

/// Render a self-contained HTML report with findings, health scores, and source snippets.
pub fn render_html(
    findings: &[Finding],
    scores: &[HealthScore],
    file_contents: &[(String, String)],
) -> String {
    let mut html = String::with_capacity(32_000);
    let _ = write!(
        html,
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
        <title>Cha Report</title><style>{CSS}</style></head><body>"
    );
    render_summary(&mut html, findings, scores);
    render_scores_table(&mut html, scores);
    let contents: BTreeMap<&str, &str> = file_contents
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    render_findings_section(&mut html, findings, &contents);
    let _ = write!(html, "</body></html>");
    html
}

fn render_summary(html: &mut String, findings: &[Finding], scores: &[HealthScore]) {
    let (errors, warnings, hints) = count_severities(findings);
    let total_debt: u32 = scores.iter().map(|s| s.debt_minutes).sum();
    let _ = write!(
        html,
        "<header><h1>察 Cha Report</h1>\
         <div class=\"summary\">\
         <span class=\"badge error\">{errors} error</span>\
         <span class=\"badge warning\">{warnings} warning</span>\
         <span class=\"badge hint\">{hints} hint</span>\
         <span class=\"badge debt\">~{} debt</span>\
         </div></header>",
        format_duration(total_debt)
    );
}

fn render_scores_table(html: &mut String, scores: &[HealthScore]) {
    let _ = write!(
        html,
        "<section><h2>Health Scores</h2><table><tr>\
        <th>Grade</th><th>File</th><th>Debt</th><th>Lines</th></tr>"
    );
    for s in scores {
        let _ = write!(
            html,
            "<tr class=\"grade-{g}\"><td class=\"grade\">{g}</td>\
             <td><a href=\"#f-{id}\">{path}</a></td>\
             <td>~{d}min</td><td>{l}</td></tr>",
            g = s.grade,
            id = path_id(&s.path),
            path = esc(&s.path),
            d = s.debt_minutes,
            l = s.lines,
        );
    }
    let _ = write!(html, "</table></section>");
}

fn render_findings_section(
    html: &mut String,
    findings: &[Finding],
    contents: &BTreeMap<&str, &str>,
) {
    let grouped = group_by_file(findings);
    let _ = write!(html, "<section><h2>Findings</h2>");
    for (path, file_findings) in &grouped {
        let _ = write!(
            html,
            "<details open id=\"f-{id}\"><summary><strong>{path}</strong> \
             <span class=\"count\">({n})</span></summary>",
            id = path_id(path),
            path = esc(path),
            n = file_findings.len(),
        );
        if let Some(src) = contents.get(path.as_str()) {
            render_source_block(html, path, src, file_findings);
        }
        for f in file_findings {
            let _ = write!(
                html,
                "<div class=\"finding {sev}\"><span class=\"sev\">{icon}</span> \
                 <a href=\"#{id}-L{line}\">[{name}] L{start}-{end}</a> {msg}</div>",
                sev = sev_class(f.severity),
                icon = sev_icon(f.severity),
                id = path_id(path),
                line = f.location.start_line,
                name = esc(&f.smell_name),
                start = f.location.start_line,
                end = f.location.end_line,
                msg = esc(&f.message),
            );
        }
        let _ = write!(html, "</details>");
    }
    let _ = write!(html, "</section>");
}

fn render_source_block(html: &mut String, path: &str, src: &str, file_findings: &[&Finding]) {
    let highlight_lines = finding_lines(file_findings);
    let _ = write!(html, "<div class=\"source\"><table class=\"code\">");
    for (i, line) in src.lines().enumerate() {
        let ln = i + 1;
        let cls = if highlight_lines.contains(&ln) {
            " class=\"hl\""
        } else {
            ""
        };
        let _ = write!(
            html,
            "<tr{cls} id=\"{id}-L{ln}\"><td class=\"ln\">{ln}</td>\
             <td class=\"src\">{code}</td></tr>",
            id = path_id(path),
            code = esc(line),
        );
    }
    let _ = write!(html, "</table></div>");
}

fn count_severities(findings: &[Finding]) -> (usize, usize, usize) {
    let mut e = 0;
    let mut w = 0;
    let mut h = 0;
    for f in findings {
        match f.severity {
            Severity::Error => e += 1,
            Severity::Warning => w += 1,
            Severity::Hint => h += 1,
        }
    }
    (e, w, h)
}

fn group_by_file(findings: &[Finding]) -> BTreeMap<String, Vec<&Finding>> {
    let mut map: BTreeMap<String, Vec<&Finding>> = BTreeMap::new();
    for f in findings {
        map.entry(f.location.path.to_string_lossy().to_string())
            .or_default()
            .push(f);
    }
    map
}

fn finding_lines(findings: &[&Finding]) -> std::collections::HashSet<usize> {
    let mut set = std::collections::HashSet::new();
    for f in findings {
        for l in f.location.start_line..=f.location.end_line {
            set.insert(l);
        }
    }
    set
}

fn path_id(path: &str) -> String {
    path.replace(['/', '\\', '.'], "-")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn sev_class(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Hint => "hint",
    }
}

fn sev_icon(s: Severity) -> &'static str {
    match s {
        Severity::Error => "🔴",
        Severity::Warning => "⚠️",
        Severity::Hint => "ℹ️",
    }
}

fn format_duration(minutes: u32) -> String {
    if minutes < 60 {
        format!("{minutes}min")
    } else {
        let h = minutes / 60;
        let m = minutes % 60;
        if m == 0 {
            format!("{h}h")
        } else {
            format!("{h}h {m}min")
        }
    }
}

const CSS: &str = r#"
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,monospace;background:#0d1117;color:#c9d1d9;padding:2rem;max-width:1200px;margin:0 auto}
header{margin-bottom:2rem}
h1{font-size:1.8rem;margin-bottom:.5rem;color:#58a6ff}
h2{font-size:1.3rem;margin:1.5rem 0 .8rem;color:#8b949e;border-bottom:1px solid #21262d;padding-bottom:.3rem}
.summary{display:flex;gap:.5rem;flex-wrap:wrap}
.badge{padding:.2rem .6rem;border-radius:4px;font-size:.85rem;font-weight:600}
.badge.error{background:#da3633;color:#fff}
.badge.warning{background:#d29922;color:#fff}
.badge.hint{background:#388bfd;color:#fff}
.badge.debt{background:#21262d;color:#8b949e}
table{width:100%;border-collapse:collapse;margin:.5rem 0}
th,td{text-align:left;padding:.3rem .6rem;border-bottom:1px solid #21262d}
th{color:#8b949e;font-size:.8rem;text-transform:uppercase}
.grade{font-weight:700;font-size:1.1rem}
.grade-A .grade{color:#3fb950}.grade-B .grade{color:#58a6ff}.grade-C .grade{color:#d29922}.grade-D .grade{color:#f85149}.grade-F .grade{color:#da3633}
details{margin:.8rem 0;background:#161b22;border:1px solid #21262d;border-radius:6px;overflow:hidden}
summary{padding:.6rem 1rem;cursor:pointer;background:#161b22}
summary:hover{background:#1c2128}
.count{color:#8b949e;font-weight:400}
.source{overflow-x:auto;max-height:400px;overflow-y:auto}
.code{font-size:.8rem;width:100%}
.code td{border:none;padding:0 .5rem;white-space:pre}
.code .ln{color:#484f58;text-align:right;user-select:none;width:3rem;min-width:3rem}
.code .src{color:#c9d1d9}
.code tr.hl{background:#2d1b00}
.code tr.hl .src{color:#f0c674}
.finding{padding:.4rem 1rem;font-size:.85rem;border-left:3px solid}
.finding.error{border-color:#da3633;background:#1a0000}.finding.warning{border-color:#d29922;background:#1a1500}.finding.hint{border-color:#388bfd;background:#0a1929}
.finding a{color:#58a6ff;text-decoration:none}
.finding a:hover{text-decoration:underline}
.sev{margin-right:.3rem}
"#;
