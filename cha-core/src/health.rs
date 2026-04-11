use crate::config::DebtWeights;
use crate::{Finding, Severity};

/// Health grade A–F for a file, based on issue density.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
    A,
    B,
    C,
    D,
    F,
}

impl std::fmt::Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Grade::A => "A",
            Grade::B => "B",
            Grade::C => "C",
            Grade::D => "D",
            Grade::F => "F",
        })
    }
}

/// Per-file health score.
#[derive(Debug, Clone)]
pub struct HealthScore {
    pub path: String,
    pub grade: Grade,
    /// Estimated remediation cost in minutes.
    pub debt_minutes: u32,
    pub lines: usize,
}

/// Compute health scores grouped by file.
pub fn score_files(
    findings: &[Finding],
    file_lines: &[(String, usize)],
    weights: &DebtWeights,
) -> Vec<HealthScore> {
    use std::collections::HashMap;
    let mut debt: HashMap<String, u32> = HashMap::new();
    for f in findings {
        let path = f.location.path.to_string_lossy().to_string();
        *debt.entry(path).or_default() += debt_minutes(f.severity, weights);
    }
    file_lines
        .iter()
        .map(|(path, lines)| {
            let mins = debt.get(path).copied().unwrap_or(0);
            let grade = rate(mins, *lines);
            HealthScore {
                path: path.clone(),
                grade,
                debt_minutes: mins,
                lines: *lines,
            }
        })
        .collect()
}

/// Estimated fix time per severity (minutes).
fn debt_minutes(s: Severity, w: &DebtWeights) -> u32 {
    match s {
        Severity::Hint => w.hint,
        Severity::Warning => w.warning,
        Severity::Error => w.error,
    }
}

/// Grade based on debt ratio (minutes per 100 lines).
/// A: ≤5, B: ≤10, C: ≤20, D: ≤40, F: >40
fn rate(debt_mins: u32, lines: usize) -> Grade {
    if lines == 0 {
        return Grade::A;
    }
    let ratio = (debt_mins as f64) / (lines as f64) * 100.0;
    if ratio <= 5.0 {
        Grade::A
    } else if ratio <= 10.0 {
        Grade::B
    } else if ratio <= 20.0 {
        Grade::C
    } else if ratio <= 40.0 {
        Grade::D
    } else {
        Grade::F
    }
}
