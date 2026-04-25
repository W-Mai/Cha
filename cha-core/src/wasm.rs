use std::collections::HashMap;
use std::path::{Path, PathBuf};

use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::plugin::{Finding, Location, Severity, SmellCategory};
use crate::{AnalysisContext, Plugin};

mod bindings {
    wasmtime::component::bindgen!({
        path: "wit/plugin.wit",
        world: "analyzer",
    });
}

use bindings::Analyzer;
pub use bindings::cha::plugin::types as wit;

struct HostState {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

fn new_host_state() -> HostState {
    let wasi = WasiCtxBuilder::new().build();
    HostState {
        wasi,
        table: ResourceTable::new(),
    }
}

/// Adapter that loads a WASM component and wraps it as a Plugin.
pub struct WasmPlugin {
    engine: Engine,
    component: Component,
    plugin_name: String,
    plugin_version: String,
    plugin_description: String,
    plugin_authors: Vec<String>,
    plugin_smells: Vec<String>,
    options: Vec<(String, wit::OptionValue)>,
}

impl WasmPlugin {
    pub fn load(path: &Path) -> wasmtime::Result<Self> {
        let engine = Engine::default();
        let bytes = std::fs::read(path)?;
        let component = Component::from_binary(&engine, &bytes)?;

        let mut linker = Linker::<HostState>::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

        let mut store = Store::new(&engine, new_host_state());
        let instance = Analyzer::instantiate(&mut store, &component, &linker)?;
        let name = instance.call_name(&mut store)?;
        let version = instance.call_version(&mut store)?;
        let description = instance.call_description(&mut store)?;
        let authors = instance.call_authors(&mut store)?;
        let smells = instance.call_smells(&mut store)?;

        Ok(Self {
            engine,
            component,
            plugin_name: name,
            plugin_version: version,
            plugin_description: description,
            plugin_authors: authors,
            plugin_smells: smells,
            options: vec![],
        })
    }

    /// Set plugin options from config.
    pub fn set_options(&mut self, options: Vec<(String, wit::OptionValue)>) {
        self.options = options;
    }
}

impl Plugin for WasmPlugin {
    fn name(&self) -> &str {
        &self.plugin_name
    }

    fn version(&self) -> &str {
        &self.plugin_version
    }

    fn description(&self) -> &str {
        &self.plugin_description
    }

    fn authors(&self) -> Vec<String> {
        self.plugin_authors.clone()
    }

    fn smells(&self) -> Vec<String> {
        self.plugin_smells.clone()
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let result = (|| -> wasmtime::Result<Vec<Finding>> {
            let mut linker = Linker::<HostState>::new(&self.engine);
            wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

            let mut store = Store::new(&self.engine, new_host_state());
            let instance = Analyzer::instantiate(&mut store, &self.component, &linker)?;
            let input = to_wit_input(ctx, &self.options);
            let results = instance.call_analyze(&mut store, &input)?;
            Ok(results.into_iter().map(from_wit_finding).collect())
        })();

        result.unwrap_or_else(|e| {
            eprintln!("wasm plugin error: {}", e);
            vec![]
        })
    }
}

fn to_wit_input(
    ctx: &AnalysisContext,
    options: &[(String, wit::OptionValue)],
) -> wit::AnalysisInput {
    wit::AnalysisInput {
        path: ctx.file.path.to_string_lossy().into(),
        content: ctx.file.content.clone(),
        language: ctx.model.language.clone(),
        total_lines: ctx.model.total_lines as u32,
        functions: convert_functions(&ctx.model.functions),
        classes: convert_classes(&ctx.model.classes),
        imports: convert_imports(&ctx.model.imports),
        options: options.to_vec(),
    }
}

/// Generic slice converter to avoid duplicate map-collect patterns.
fn convert_slice<T, U>(items: &[T], f: impl Fn(&T) -> U) -> Vec<U> {
    items.iter().map(f).collect()
}

fn convert_functions(funcs: &[crate::model::FunctionInfo]) -> Vec<wit::FunctionInfo> {
    convert_slice(funcs, |f| wit::FunctionInfo {
        name: f.name.clone(),
        start_line: f.start_line as u32,
        end_line: f.end_line as u32,
        name_col: f.name_col as u32,
        name_end_col: f.name_end_col as u32,
        line_count: f.line_count as u32,
        complexity: f.complexity as u32,
        parameter_count: f.parameter_count as u32,
        parameter_types: f.parameter_types.iter().map(to_wit_type_ref).collect(),
        chain_depth: f.chain_depth as u32,
        switch_arms: f.switch_arms as u32,
        external_refs: f.external_refs.clone(),
        is_delegating: f.is_delegating,
        is_exported: f.is_exported,
        comment_lines: f.comment_lines as u32,
        referenced_fields: f.referenced_fields.clone(),
        null_check_fields: f.null_check_fields.clone(),
        switch_dispatch_target: f.switch_dispatch_target.clone(),
        optional_param_count: f.optional_param_count as u32,
        called_functions: f.called_functions.clone(),
        cognitive_complexity: f.cognitive_complexity as u32,
        body_hash: f.body_hash.map(|h| format!("{h:016x}")),
        return_type: f.return_type.as_ref().map(to_wit_type_ref),
    })
}

fn to_wit_type_ref(t: &crate::model::TypeRef) -> wit::TypeRef {
    wit::TypeRef {
        name: t.name.clone(),
        raw: t.raw.clone(),
        origin: match &t.origin {
            crate::model::TypeOrigin::Local => wit::TypeOrigin::ProjectLocal,
            crate::model::TypeOrigin::External(m) => wit::TypeOrigin::External(m.clone()),
            crate::model::TypeOrigin::Primitive => wit::TypeOrigin::Primitive,
            crate::model::TypeOrigin::Unknown => wit::TypeOrigin::Unknown,
        },
    }
}

fn convert_classes(classes: &[crate::model::ClassInfo]) -> Vec<wit::ClassInfo> {
    convert_slice(classes, |c| wit::ClassInfo {
        name: c.name.clone(),
        start_line: c.start_line as u32,
        end_line: c.end_line as u32,
        name_col: c.name_col as u32,
        name_end_col: c.name_end_col as u32,
        method_count: c.method_count as u32,
        line_count: c.line_count as u32,
        delegating_method_count: c.delegating_method_count as u32,
        field_count: c.field_count as u32,
        field_names: c.field_names.clone(),
        field_types: c.field_types.clone(),
        is_exported: c.is_exported,
        has_behavior: c.has_behavior,
        is_interface: c.is_interface,
        parent_name: c.parent_name.clone(),
        override_count: c.override_count as u32,
        self_call_count: c.self_call_count as u32,
        has_listener_field: c.has_listener_field,
        has_notify_method: c.has_notify_method,
    })
}

fn convert_imports(imports: &[crate::model::ImportInfo]) -> Vec<wit::ImportInfo> {
    convert_slice(imports, |i| wit::ImportInfo {
        source: i.source.clone(),
        line: i.line as u32,
        col: i.col as u32,
    })
}

fn from_wit_finding(f: wit::Finding) -> Finding {
    Finding {
        smell_name: f.smell_name,
        category: convert_category(f.category),
        severity: convert_severity(f.severity),
        location: Location {
            path: PathBuf::from(&f.location.path),
            start_line: f.location.start_line as usize,
            start_col: f.location.start_col as usize,
            end_line: f.location.end_line as usize,
            end_col: f.location.end_col as usize,
            name: f.location.name,
        },
        message: f.message,
        suggested_refactorings: f.suggested_refactorings,
        actual_value: f.actual_value,
        threshold: f.threshold,
    }
}

fn convert_severity(s: wit::Severity) -> Severity {
    match s {
        wit::Severity::Hint => Severity::Hint,
        wit::Severity::Warning => Severity::Warning,
        wit::Severity::Error => Severity::Error,
    }
}

fn convert_category(c: wit::SmellCategory) -> SmellCategory {
    match c {
        wit::SmellCategory::Bloaters => SmellCategory::Bloaters,
        wit::SmellCategory::OoAbusers => SmellCategory::OoAbusers,
        wit::SmellCategory::ChangePreventers => SmellCategory::ChangePreventers,
        wit::SmellCategory::Dispensables => SmellCategory::Dispensables,
        wit::SmellCategory::Couplers => SmellCategory::Couplers,
        wit::SmellCategory::Security => SmellCategory::Security,
    }
}

/// Scan plugin directories and load all .wasm components.
pub fn load_wasm_plugins(project_dir: &Path) -> Vec<WasmPlugin> {
    let mut plugins: Vec<WasmPlugin> = Vec::new();
    let mut seen = HashMap::new();

    let project_plugins = project_dir.join(".cha").join("plugins");
    let global_plugins = home_dir().join(".cha").join("plugins");

    for dir in [&project_plugins, &global_plugins] {
        load_plugins_from_dir(dir, &mut seen, &mut plugins);
    }

    plugins
}

/// Load .wasm plugins from a single directory, skipping duplicates by filename.
fn load_plugins_from_dir(
    dir: &Path,
    seen: &mut HashMap<String, bool>,
    plugins: &mut Vec<WasmPlugin>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "wasm") {
            continue;
        }
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if seen.contains_key(&filename) {
            continue;
        }
        match WasmPlugin::load(&path) {
            Ok(p) => {
                seen.insert(filename, true);
                plugins.push(p);
            }
            Err(e) => {
                eprintln!("failed to load wasm plugin {}: {}", path.display(), e);
            }
        }
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Convert a TOML value to a WIT option-value.
pub fn toml_to_option_value(v: &toml::Value) -> Option<wit::OptionValue> {
    match v {
        toml::Value::String(s) => Some(wit::OptionValue::Str(s.clone())),
        toml::Value::Integer(i) => Some(wit::OptionValue::Int(*i)),
        toml::Value::Float(f) => Some(wit::OptionValue::Float(*f)),
        toml::Value::Boolean(b) => Some(wit::OptionValue::Boolean(*b)),
        toml::Value::Array(arr) => {
            let strs: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            Some(wit::OptionValue::ListStr(strs))
        }
        _ => None,
    }
}
