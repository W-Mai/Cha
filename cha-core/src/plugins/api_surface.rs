use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Analyze the ratio of exported (public) API surface.
pub struct ApiSurfaceAnalyzer {
    pub max_exported_ratio: f64,
    pub max_exported_count: usize,
    /// Higher thresholds for C/C++ implementation files (.c/.cpp).
    /// C modules legitimately export more symbols than typical OO classes.
    pub c_max_exported_ratio: f64,
    pub c_max_exported_count: usize,
    /// Skip C/C++ header files (.h/.hpp). Headers are public-API by design,
    /// so the "too many exported items" signal is meaningless for them.
    pub skip_c_headers: bool,
}

impl Default for ApiSurfaceAnalyzer {
    fn default() -> Self {
        Self {
            max_exported_ratio: 0.8,
            max_exported_count: 20,
            // C ratio gate effectively off — `.c` files often export 100% of
            // their non-static functions by design (the .h pairs the visibility).
            // Only the count threshold matters.
            c_max_exported_ratio: 1.01,
            c_max_exported_count: 30,
            skip_c_headers: true,
        }
    }
}

impl Plugin for ApiSurfaceAnalyzer {
    fn name(&self) -> &str {
        "api_surface"
    }

    fn smells(&self) -> Vec<String> {
        vec!["large_api_surface".into()]
    }

    fn description(&self) -> &str {
        "Exported ratio too high, narrow the public API"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let is_c_like = matches!(ctx.model.language.as_str(), "c" | "cpp");
        if is_c_like && self.skip_c_headers && is_header_file(&ctx.file.path) {
            return vec![];
        }

        let total = ctx.model.functions.len() + ctx.model.classes.len();
        if total < 5 {
            return vec![];
        }

        let exported = count_exported(ctx);
        let ratio = exported as f64 / total as f64;

        let (max_count, max_ratio) = if is_c_like {
            (self.c_max_exported_count, self.c_max_exported_ratio)
        } else {
            (self.max_exported_count, self.max_exported_ratio)
        };

        if exported > max_count || ratio > max_ratio {
            vec![self.make_finding(ctx, exported, total, ratio, max_ratio)]
        } else {
            vec![]
        }
    }
}

/// Check if path looks like a C/C++ header file.
fn is_header_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("h" | "hpp" | "hxx" | "hh" | "h++")
    )
}

/// Count total exported functions and classes.
fn count_exported(ctx: &AnalysisContext) -> usize {
    let fns = ctx.model.functions.iter().filter(|f| f.is_exported).count();
    let cls = ctx.model.classes.iter().filter(|c| c.is_exported).count();
    fns + cls
}

impl ApiSurfaceAnalyzer {
    /// Build the large API surface finding.
    fn make_finding(
        &self,
        ctx: &AnalysisContext,
        exported: usize,
        total: usize,
        ratio: f64,
        threshold: f64,
    ) -> Finding {
        Finding {
            smell_name: "large_api_surface".into(),
            category: SmellCategory::Bloaters,
            severity: Severity::Warning,
            location: Location {
                path: ctx.file.path.clone(),
                start_line: 1,
                end_line: 1,
                name: None,
                ..Default::default()
            },
            message: format!(
                "File exports {}/{} items ({:.0}%), consider narrowing the public API",
                exported,
                total,
                ratio * 100.0
            ),
            suggested_refactorings: vec!["Hide Method".into(), "Extract Class".into()],
            actual_value: Some(ratio),
            threshold: Some(threshold),
            risk_score: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionInfo, SourceFile, SourceModel};
    use std::path::PathBuf;

    fn exported_fn(name: &str) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            is_exported: true,
            start_line: 1,
            end_line: 2,
            line_count: 2,
            ..Default::default()
        }
    }

    fn make_ctx<'a>(file: &'a SourceFile, model: &'a SourceModel) -> AnalysisContext<'a> {
        AnalysisContext {
            file,
            model,
            tree: None,
            ts_language: None,
            project: None,
        }
    }

    #[test]
    fn skips_c_header_with_100_percent_exports() {
        let file = SourceFile::new(PathBuf::from("foo.h"), String::new());
        let model = SourceModel {
            language: "c".into(),
            functions: (0..30).map(|i| exported_fn(&format!("f{}", i))).collect(),
            ..Default::default()
        };
        let ctx = make_ctx(&file, &model);
        let findings = ApiSurfaceAnalyzer::default().analyze(&ctx);
        assert!(findings.is_empty(), "should skip .h files");
    }

    #[test]
    fn skips_cpp_header() {
        let file = SourceFile::new(PathBuf::from("foo.hpp"), String::new());
        let model = SourceModel {
            language: "cpp".into(),
            functions: (0..30).map(|i| exported_fn(&format!("f{}", i))).collect(),
            ..Default::default()
        };
        let ctx = make_ctx(&file, &model);
        assert!(ApiSurfaceAnalyzer::default().analyze(&ctx).is_empty());
    }

    #[test]
    fn c_impl_uses_higher_threshold() {
        let file = SourceFile::new(PathBuf::from("foo.c"), String::new());
        // 25 exported + 5 private = 30 total, ratio 0.83. Rust threshold (20/0.80) fires;
        // C threshold (30/0.95) should not.
        let mut funcs: Vec<FunctionInfo> =
            (0..25).map(|i| exported_fn(&format!("f{}", i))).collect();
        for i in 0..5 {
            let mut p = exported_fn(&format!("p{}", i));
            p.is_exported = false;
            funcs.push(p);
        }
        let model = SourceModel {
            language: "c".into(),
            functions: funcs,
            ..Default::default()
        };
        let ctx = make_ctx(&file, &model);
        let findings = ApiSurfaceAnalyzer::default().analyze(&ctx);
        assert!(
            findings.is_empty(),
            "C .c with 25/30 exports should not fire"
        );
    }

    #[test]
    fn rust_still_uses_default_threshold() {
        let file = SourceFile::new(PathBuf::from("foo.rs"), String::new());
        let model = SourceModel {
            language: "rust".into(),
            functions: (0..25).map(|i| exported_fn(&format!("f{}", i))).collect(),
            ..Default::default()
        };
        let ctx = make_ctx(&file, &model);
        let findings = ApiSurfaceAnalyzer::default().analyze(&ctx);
        assert_eq!(findings.len(), 1, "Rust 25 exports > 20 should fire");
    }

    #[test]
    fn c_impl_above_c_threshold_fires() {
        let file = SourceFile::new(PathBuf::from("foo.c"), String::new());
        let model = SourceModel {
            language: "c".into(),
            functions: (0..35).map(|i| exported_fn(&format!("f{}", i))).collect(),
            ..Default::default()
        };
        let ctx = make_ctx(&file, &model);
        let findings = ApiSurfaceAnalyzer::default().analyze(&ctx);
        assert_eq!(
            findings.len(),
            1,
            "C .c file with 35 exports > 30 should fire"
        );
    }

    #[test]
    fn skip_c_headers_can_be_disabled() {
        let file = SourceFile::new(PathBuf::from("foo.h"), String::new());
        let model = SourceModel {
            language: "c".into(),
            functions: (0..35).map(|i| exported_fn(&format!("f{}", i))).collect(),
            ..Default::default()
        };
        let ctx = make_ctx(&file, &model);
        let analyzer = ApiSurfaceAnalyzer {
            skip_c_headers: false,
            ..Default::default()
        };
        let findings = analyzer.analyze(&ctx);
        assert_eq!(findings.len(), 1, "header should fire when skip is off");
    }
}
