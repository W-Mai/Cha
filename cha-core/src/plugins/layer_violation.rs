use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect imports that violate configured layer boundaries.
/// Layers are defined as path prefixes with a numeric order.
/// Lower layers must not import from higher layers.
///
/// Configure via .cha.toml:
/// ```toml
/// [plugins.layer_violation.options]
/// layers = "domain:0,service:1,controller:2,ui:3"
/// ```
#[derive(Default)]
pub struct LayerViolationAnalyzer {
    /// Ordered layers: (prefix, level). Lower level = lower layer.
    pub layers: Vec<(String, u32)>,
}

impl Plugin for LayerViolationAnalyzer {
    fn name(&self) -> &str {
        "layer_violation"
    }

    fn smells(&self) -> Vec<String> {
        vec!["layer_violation".into()]
    }

    fn description(&self) -> &str {
        "Cross-layer dependency violation"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        if self.layers.is_empty() {
            return vec![];
        }

        let file_path = ctx.file.path.to_string_lossy();
        let file_layer = self.layer_of(&file_path);

        ctx.model
            .imports
            .iter()
            .filter_map(|imp| self.check_import(ctx, &file_layer, imp))
            .collect()
    }
}

impl LayerViolationAnalyzer {
    /// Check a single import against the file's layer boundary.
    fn check_import(
        &self,
        ctx: &AnalysisContext,
        file_layer: &Option<(String, u32)>,
        imp: &crate::ImportInfo,
    ) -> Option<Finding> {
        let import_layer = self.layer_of(&imp.source);
        let (_, file_level) = file_layer.as_ref()?;
        let (imp_name, imp_level) = import_layer.as_ref()?;
        if file_level >= imp_level {
            return None;
        }
        Some(Finding {
            smell_name: "layer_violation".into(),
            category: SmellCategory::Couplers,
            severity: Severity::Error,
            location: Location {
                path: ctx.file.path.clone(),
                start_line: imp.line,
                start_col: imp.col,
                end_line: imp.line,
                name: None,
                ..Default::default()
            },
            message: format!(
                "Import `{}` violates layer boundary (importing from layer `{}` into lower layer)",
                imp.source, imp_name
            ),
            suggested_refactorings: vec!["Move Method".into(), "Extract Interface".into()],
            ..Default::default()
        })
    }

    fn layer_of(&self, path: &str) -> Option<(String, u32)> {
        self.layers
            .iter()
            .find(|(prefix, _)| path.contains(prefix.as_str()))
            .map(|(name, level)| (name.clone(), *level))
    }

    /// Parse layers from config string: "domain:0,service:1,controller:2"
    pub fn from_config_str(s: &str) -> Self {
        let layers = s
            .split(',')
            .filter_map(|part| {
                let mut parts = part.trim().splitn(2, ':');
                let name = parts.next()?.trim().to_string();
                let level = parts.next()?.trim().parse().ok()?;
                Some((name, level))
            })
            .collect();
        Self { layers }
    }
}
