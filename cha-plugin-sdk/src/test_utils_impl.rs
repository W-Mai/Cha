#[cfg(feature = "test-utils")]
pub mod test_utils {
    use anyhow::{Context, Result};
    use cha_core::{AnalysisContext, Finding, Plugin, SourceFile, wasm::WasmPlugin};
    use std::path::PathBuf;

    /// Builder for WASM plugin integration tests.
    ///
    /// Automatically builds the plugin if the wasm file is missing.
    ///
    /// # Example
    /// ```rust,ignore
    /// WasmPluginTest::new()
    ///     .source("typescript", "function todo_fix() {}")
    ///     .assert_finding("suspicious_name");
    /// ```
    pub struct WasmPluginTest {
        wasm_path: PathBuf,
        language: String,
        source: String,
        options: Vec<(String, String)>,
    }

    impl WasmPluginTest {
        /// Use the default output path from `cha plugin build` in the current directory.
        pub fn new() -> Self {
            let name = read_package_name().unwrap_or_else(|| "plugin".into());
            Self {
                wasm_path: PathBuf::from(format!("{name}.wasm")),
                language: String::new(),
                source: String::new(),
                options: vec![],
            }
        }

        /// Specify a custom wasm file path.
        pub fn from_file(path: impl Into<PathBuf>) -> Self {
            Self {
                wasm_path: path.into(),
                language: String::new(),
                source: String::new(),
                options: vec![],
            }
        }

        /// Set the source code and language to analyze.
        pub fn source(mut self, language: &str, code: &str) -> Self {
            self.language = language.into();
            self.source = code.into();
            self
        }

        /// Add a string option passed to the plugin.
        pub fn option(mut self, key: &str, value: &str) -> Self {
            self.options.push((key.into(), value.into()));
            self
        }

        fn build_and_load(&self) -> Result<(WasmPlugin, SourceFile)> {
            if !self.wasm_path.exists() {
                build_plugin().context("auto-build failed; run `cha plugin build` manually")?;
            }
            let mut plugin = WasmPlugin::load(&self.wasm_path)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("failed to load {}", self.wasm_path.display()))?;

            if !self.options.is_empty() {
                use cha_core::wasm::toml_to_option_value;
                let opts = self.options.iter().filter_map(|(k, v)| {
                    let tv = toml::Value::String(v.clone());
                    toml_to_option_value(&tv).map(|ov| (k.clone(), ov))
                }).collect();
                plugin.set_options(opts);
            }

            let ext = match self.language.as_str() {
                "typescript" | "ts" => "ts",
                "rust" | "rs" => "rs",
                other => other,
            };
            let fake_path = PathBuf::from(format!("test_input.{ext}"));
            let file = SourceFile::new(fake_path, self.source.clone());
            Ok((plugin, file))
        }

        fn run(&self) -> Vec<Finding> {
            let (plugin, file) = self.build_and_load().expect("failed to load plugin");
            let model = cha_parser::parse_file(&file).unwrap_or_else(|| cha_core::SourceModel {
                language: self.language.clone(),
                total_lines: self.source.lines().count(),
                functions: vec![],
                classes: vec![],
                imports: vec![],
                comments: vec![],
            });
            let ctx = AnalysisContext { file: &file, model: &model };
            plugin.analyze(&ctx)
        }

        /// Assert at least one finding exists.
        pub fn assert_any_finding(self) {
            let findings = self.run();
            assert!(!findings.is_empty(), "expected at least one finding, got none");
        }

        /// Assert no findings.
        pub fn assert_no_finding(self) {
            let findings = self.run();
            assert!(findings.is_empty(), "expected no findings, got: {findings:#?}");
        }

        /// Assert at least one finding with the given smell name.
        pub fn assert_finding(self, smell_name: &str) {
            let findings = self.run();
            assert!(
                findings.iter().any(|f| f.smell_name == smell_name),
                "expected finding `{smell_name}`, got: {:#?}",
                findings.iter().map(|f| &f.smell_name).collect::<Vec<_>>()
            );
        }

        /// Assert no finding with the given smell name.
        pub fn assert_no_finding_named(self, smell_name: &str) {
            let findings = self.run();
            assert!(
                !findings.iter().any(|f| f.smell_name == smell_name),
                "expected no finding `{smell_name}`, but got one"
            );
        }

        /// Return all findings for custom assertions.
        pub fn findings(self) -> Vec<Finding> {
            self.run()
        }
    }

    impl Default for WasmPluginTest {
        fn default() -> Self {
            Self::new()
        }
    }

    fn build_plugin() -> Result<()> {
        let status = std::process::Command::new("cha")
            .args(["plugin", "build"])
            .status()
            .context("failed to run `cha plugin build`")?;
        anyhow::ensure!(status.success(), "`cha plugin build` failed");
        Ok(())
    }

    fn read_package_name() -> Option<String> {
        let content = std::fs::read_to_string("Cargo.toml").ok()?;
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("name") {
                let val = rest.trim_start_matches([' ', '=', '"']).trim_end_matches('"');
                if !val.is_empty() {
                    return Some(val.replace('-', "_"));
                }
            }
        }
        None
    }
}
