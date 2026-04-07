use std::collections::HashMap;
use std::path::{Path, PathBuf};

use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::plugin::{Finding, Location, Severity, SmellCategory};
use crate::{AnalysisContext, Plugin};

mod bindings {
    wasmtime::component::bindgen!({
        path: "../wit/plugin.wit",
        world: "analyzer",
    });
}

use bindings::Analyzer;
use bindings::cha::plugin::types as wit;

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

        Ok(Self {
            engine,
            component,
            plugin_name: name,
        })
    }
}

impl Plugin for WasmPlugin {
    fn name(&self) -> &str {
        &self.plugin_name
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let result = (|| -> wasmtime::Result<Vec<Finding>> {
            let mut linker = Linker::<HostState>::new(&self.engine);
            wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

            let mut store = Store::new(&self.engine, new_host_state());
            let instance = Analyzer::instantiate(&mut store, &self.component, &linker)?;
            let input = to_wit_input(ctx);
            let results = instance.call_analyze(&mut store, &input)?;
            Ok(results.into_iter().map(from_wit_finding).collect())
        })();

        result.unwrap_or_else(|e| {
            eprintln!("wasm plugin error: {}", e);
            vec![]
        })
    }
}

fn to_wit_input(ctx: &AnalysisContext) -> wit::AnalysisInput {
    wit::AnalysisInput {
        path: ctx.file.path.to_string_lossy().into(),
        content: ctx.file.content.clone(),
        language: ctx.model.language.clone(),
        total_lines: ctx.model.total_lines as u32,
        functions: convert_functions(&ctx.model.functions),
        classes: convert_classes(&ctx.model.classes),
        imports: convert_imports(&ctx.model.imports),
        options: vec![],
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
        line_count: f.line_count as u32,
        complexity: f.complexity as u32,
    })
}

fn convert_classes(classes: &[crate::model::ClassInfo]) -> Vec<wit::ClassInfo> {
    convert_slice(classes, |c| wit::ClassInfo {
        name: c.name.clone(),
        start_line: c.start_line as u32,
        end_line: c.end_line as u32,
        method_count: c.method_count as u32,
        line_count: c.line_count as u32,
    })
}

fn convert_imports(imports: &[crate::model::ImportInfo]) -> Vec<wit::ImportInfo> {
    convert_slice(imports, |i| wit::ImportInfo {
        source: i.source.clone(),
        line: i.line as u32,
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
            end_line: f.location.end_line as usize,
            name: f.location.name,
        },
        message: f.message,
        suggested_refactorings: f.suggested_refactorings,
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
    }
}

/// Scan plugin directories and load all .wasm components.
pub fn load_wasm_plugins(project_dir: &Path) -> Vec<Box<dyn Plugin>> {
    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();
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
    plugins: &mut Vec<Box<dyn Plugin>>,
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
                plugins.push(Box::new(p));
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
