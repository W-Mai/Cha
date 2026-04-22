use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

use cha_core::{
    AnalysisContext, Config, Finding, PluginRegistry, ProjectCache, Severity, SourceFile,
    SourceModel,
};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct ChaLsp {
    client: Client,
    registry: Arc<PluginRegistry>,
    cwd: PathBuf,
    docs: Arc<RwLock<HashMap<Url, String>>>,
    disabled_plugins: Arc<RwLock<Vec<String>>>,
    cache: Arc<Mutex<ProjectCache>>,
    /// All findings from last full analyze, keyed by file URI.
    findings: Arc<RwLock<HashMap<Url, Vec<Finding>>>>,
    /// All models from last full analyze, keyed by file URI.
    models: Arc<RwLock<HashMap<Url, SourceModel>>>,
}

/// Run full workspace analyze: scan all files, use ProjectCache, populate findings + models.
fn run_full_analyze(
    cwd: &std::path::Path,
    registry: &PluginRegistry,
    cache: &Mutex<ProjectCache>,
    disabled: &[String],
) -> (HashMap<Url, Vec<Finding>>, HashMap<Url, SourceModel>) {
    let mut all_findings: HashMap<Url, Vec<Finding>> = HashMap::new();
    let mut all_models: HashMap<Url, SourceModel> = HashMap::new();
    let exts = [
        "rs", "ts", "tsx", "py", "go", "c", "h", "cpp", "cc", "cxx", "hpp",
    ];
    let walker = ignore::WalkBuilder::new(cwd)
        .hidden(true)
        .git_ignore(true)
        .build();
    let mut c = cache.lock().unwrap();
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.into_path();
        if !path
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| exts.contains(&x))
        {
            continue;
        }
        let rel = path
            .strip_prefix(cwd)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        // Try mtime fast path for model
        let (content, chash, model) =
            if let cha_core::FileStatus::Unchanged(ch) = c.check_file(&rel, &path) {
                if let Some(m) = c.get_model(ch) {
                    (None, ch, m)
                } else {
                    let content = match std::fs::read_to_string(&path) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let ch2 = cha_core::hash_content(&content);
                    let file = SourceFile::new(path.clone(), content.clone());
                    let Some(m) = cha_parser::parse_file(&file) else {
                        continue;
                    };
                    c.put_model(ch2, &m);
                    c.update_file_entry(
                        rel.clone(),
                        &path,
                        ch2,
                        m.imports.iter().map(|i| i.source.clone()).collect(),
                    );
                    (Some(content), ch2, m)
                }
            } else {
                let content = match std::fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let ch = cha_core::hash_content(&content);
                if let Some(m) = c.get_model(ch) {
                    c.update_file_entry(
                        rel.clone(),
                        &path,
                        ch,
                        m.imports.iter().map(|i| i.source.clone()).collect(),
                    );
                    (Some(content), ch, m)
                } else {
                    let file = SourceFile::new(path.clone(), content.clone());
                    let Some(m) = cha_parser::parse_file(&file) else {
                        continue;
                    };
                    c.put_model(ch, &m);
                    c.update_file_entry(
                        rel.clone(),
                        &path,
                        ch,
                        m.imports.iter().map(|i| i.source.clone()).collect(),
                    );
                    (Some(content), ch, m)
                }
            };

        // Get findings from cache or run plugins
        let findings = if let Some(cached) = c.get_findings(chash) {
            cached
        } else {
            let content =
                content.unwrap_or_else(|| std::fs::read_to_string(&path).unwrap_or_default());
            let file = SourceFile::new(path.clone(), content);
            let ctx = AnalysisContext {
                file: &file,
                model: &model,
            };
            let findings: Vec<Finding> = registry
                .plugins()
                .iter()
                .flat_map(|p| p.analyze(&ctx))
                .collect();
            c.put_findings(chash, &findings);
            findings
        };

        let uri = Url::from_file_path(&path)
            .unwrap_or_else(|_| Url::parse(&format!("file://{}", path.display())).unwrap());
        let filtered: Vec<Finding> = findings
            .into_iter()
            .filter(|f| !disabled.iter().any(|d| d == &f.smell_name))
            .collect();
        all_findings.insert(uri.clone(), filtered);
        all_models.insert(uri, model);
    }
    c.flush();
    (all_findings, all_models)
}

/// Build semantic tokens for functions/classes with warnings (modifier bit 0 = "warning").
fn build_semantic_tokens(
    model: &SourceModel,
    warn_lines: &std::collections::HashSet<usize>,
) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut prev_line = 0u32;
    // Collect (line, col_len, type_index, modifier) sorted by line
    let mut entries: Vec<(u32, u32, u32, u32)> = Vec::new();
    for f in &model.functions {
        let line = f.start_line.saturating_sub(1) as u32;
        let has_warn = (f.start_line..=f.end_line).any(|l| warn_lines.contains(&l));
        entries.push((line, f.name.len() as u32, 0, if has_warn { 1 } else { 0 }));
    }
    for c in &model.classes {
        let line = c.start_line.saturating_sub(1) as u32;
        let has_warn = (c.start_line..=c.end_line).any(|l| warn_lines.contains(&l));
        entries.push((line, c.name.len() as u32, 1, if has_warn { 1 } else { 0 }));
    }
    entries.sort_by_key(|e| e.0);
    for (line, length, token_type, modifiers) in entries {
        tokens.push(SemanticToken {
            delta_line: line - prev_line,
            delta_start: 0,
            length,
            token_type,
            token_modifiers_bitset: modifiers,
        });
        prev_line = line;
    }
    tokens
}

#[allow(deprecated)]
fn make_fn_symbol(
    f: &cha_core::FunctionInfo,
    warn_lines: &std::collections::HashSet<usize>,
) -> DocumentSymbol {
    let start = f.start_line.saturating_sub(1) as u32;
    let end = f.end_line.saturating_sub(1) as u32;
    let icon = if (f.start_line..=f.end_line).any(|l| warn_lines.contains(&l)) {
        "⚠ "
    } else {
        ""
    };
    DocumentSymbol {
        name: format!("{icon}{}", f.name),
        detail: Some(format!("cx:{} {}L", f.complexity, f.line_count)),
        kind: SymbolKind::FUNCTION,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position::new(start, 0),
            end: Position::new(end, 0),
        },
        selection_range: Range {
            start: Position::new(start, 0),
            end: Position::new(start, 0),
        },
        children: None,
    }
}

#[allow(deprecated)]
fn make_class_symbol(
    c: &cha_core::ClassInfo,
    warn_lines: &std::collections::HashSet<usize>,
) -> DocumentSymbol {
    let start = c.start_line.saturating_sub(1) as u32;
    let end = c.end_line.saturating_sub(1) as u32;
    let icon = if (c.start_line..=c.end_line).any(|l| warn_lines.contains(&l)) {
        "⚠ "
    } else {
        ""
    };
    DocumentSymbol {
        name: format!("{icon}{}", c.name),
        detail: Some(format!(
            "{}m {}f {}L",
            c.method_count, c.field_count, c.line_count
        )),
        kind: SymbolKind::CLASS,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position::new(start, 0),
            end: Position::new(end, 0),
        },
        selection_range: Range {
            start: Position::new(start, 0),
            end: Position::new(start, 0),
        },
        children: None,
    }
}

fn finding_to_diagnostic(f: &Finding) -> Diagnostic {
    let severity = match f.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Hint => DiagnosticSeverity::HINT,
    };

    let start = f.location.start_line.saturating_sub(1);
    let end = f.location.end_line.saturating_sub(1);

    Diagnostic {
        range: Range {
            start: Position::new(start as u32, 0),
            end: Position::new(end as u32, 0),
        },
        severity: Some(severity),
        source: Some("cha".into()),
        code: Some(NumberOrString::String(f.smell_name.clone())),
        message: f.message.clone(),
        data: if f.suggested_refactorings.is_empty() {
            None
        } else {
            Some(serde_json::json!(f.suggested_refactorings))
        },
        ..Default::default()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ChaLsp {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(opts) = params.initialization_options
            && let Some(disabled) = opts.get("disabledPlugins")
            && let Ok(list) = serde_json::from_value::<Vec<String>>(disabled.clone())
        {
            *self.disabled_plugins.write().await = list;
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::CLASS,
                                ],
                                token_modifiers: vec![SemanticTokenModifier::new("warning")],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("cha".into()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: true,
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cha-lsp initialized")
            .await;
        let disabled = self.disabled_plugins.read().await.clone();
        let (f, m) = run_full_analyze(&self.cwd, &self.registry, &self.cache, &disabled);
        *self.findings.write().await = f;
        *self.models.write().await = m;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        self.docs.write().await.insert(uri, text);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.docs
                .write()
                .await
                .insert(params.text_document.uri.clone(), text);
        }
        let disabled = self.disabled_plugins.read().await.clone();
        let (f, m) = run_full_analyze(&self.cwd, &self.registry, &self.cache, &disabled);
        *self.findings.write().await = f;
        *self.models.write().await = m;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.docs
                .write()
                .await
                .insert(params.text_document.uri.clone(), change.text.clone());
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let docs = self.docs.read().await;
        let doc_text = docs.get(uri);

        let mut actions = Vec::new();
        collect_diagnostic_actions(&mut actions, uri, &params.context.diagnostics, doc_text);
        collect_selection_actions(&mut actions, uri, &params.range, doc_text);

        Ok(if actions.is_empty() {
            None
        } else {
            Some(actions)
        })
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = &params.text_document.uri;
        let models = self.models.read().await;
        let Some(model) = models.get(uri) else {
            return Ok(None);
        };
        let findings_map = self.findings.read().await;
        let findings = findings_map.get(uri);
        let mut lenses = Vec::new();
        for f in &model.functions {
            let line = f.start_line.saturating_sub(1) as u32;
            let count = findings
                .map(|fs| {
                    fs.iter()
                        .filter(|fd| {
                            fd.location.start_line <= f.end_line
                                && fd.location.end_line >= f.start_line
                        })
                        .count()
                })
                .unwrap_or(0);
            let title = if count > 0 {
                format!("⚠ {} issue(s) | {} lines", count, f.line_count)
            } else {
                format!("✓ {} lines", f.line_count)
            };
            lenses.push(CodeLens {
                range: Range {
                    start: Position::new(line, 0),
                    end: Position::new(line, 0),
                },
                command: Some(Command {
                    title,
                    command: String::new(),
                    arguments: None,
                }),
                data: None,
            });
        }
        for c in &model.classes {
            let line = c.start_line.saturating_sub(1) as u32;
            let count = findings
                .map(|fs| {
                    fs.iter()
                        .filter(|fd| {
                            fd.location.start_line <= c.end_line
                                && fd.location.end_line >= c.start_line
                        })
                        .count()
                })
                .unwrap_or(0);
            let title = if count > 0 {
                format!("⚠ {} issue(s) | {} lines", count, c.line_count)
            } else {
                format!(
                    "✓ {} methods | {} fields | {} lines",
                    c.method_count, c.field_count, c.line_count
                )
            };
            lenses.push(CodeLens {
                range: Range {
                    start: Position::new(line, 0),
                    end: Position::new(line, 0),
                },
                command: Some(Command {
                    title,
                    command: String::new(),
                    arguments: None,
                }),
                data: None,
            });
        }
        Ok(Some(lenses))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let line = pos.line as usize + 1;
        let models = self.models.read().await;
        let Some(model) = models.get(uri) else {
            return Ok(None);
        };
        let findings_map = self.findings.read().await;
        let findings = findings_map.get(uri);

        for f in &model.functions {
            if line >= f.start_line && line <= f.end_line {
                let fn_findings: Vec<&Finding> = findings
                    .map(|fs| {
                        fs.iter()
                            .filter(|fd| {
                                fd.location.start_line <= f.end_line
                                    && fd.location.end_line >= f.start_line
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut card = format!(
                    "### 📊 `{}`\n\n\
                     | Metric | Value |\n|---|---|\n\
                     | Lines | {} |\n\
                     | Cyclomatic complexity | {} |\n\
                     | Cognitive complexity | {} |\n\
                     | Parameters | {} |\n\
                     | Chain depth | {} |",
                    f.name,
                    f.line_count,
                    f.complexity,
                    f.cognitive_complexity,
                    f.parameter_count,
                    f.chain_depth,
                );
                if !fn_findings.is_empty() {
                    card.push_str("\n\n**Findings:**\n");
                    for fd in &fn_findings {
                        let icon = match fd.severity {
                            Severity::Error => "❌",
                            Severity::Warning => "⚠️",
                            Severity::Hint => "💡",
                        };
                        card.push_str(&format!("- {icon} `{}`: {}\n", fd.smell_name, fd.message));
                    }
                }
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: card,
                    }),
                    range: None,
                }));
            }
        }
        Ok(None)
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let models = self.models.read().await;
        let Some(model) = models.get(uri) else {
            return Ok(None);
        };
        let findings_map = self.findings.read().await;
        let findings = findings_map.get(uri);
        let docs = self.docs.read().await;
        let lines: Vec<&str> = docs
            .get(uri)
            .map(|t| t.lines().collect())
            .unwrap_or_default();
        let mut hints = Vec::new();
        for f in &model.functions {
            let line = f.start_line.saturating_sub(1);
            if line >= lines.len() {
                continue;
            }
            let col = lines[line].len() as u32;
            let count = findings
                .map(|fs| {
                    fs.iter()
                        .filter(|fd| {
                            fd.location.start_line <= f.end_line
                                && fd.location.end_line >= f.start_line
                        })
                        .count()
                })
                .unwrap_or(0);
            let label = if count > 0 {
                format!("  ⚠{count}")
            } else {
                "  ✓".to_string()
            };
            hints.push(InlayHint {
                position: Position::new(line as u32, col),
                label: InlayHintLabel::String(label),
                kind: Some(InlayHintKind::PARAMETER),
                text_edits: None,
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: None,
            });
        }
        Ok(Some(hints))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let models = self.models.read().await;
        let Some(model) = models.get(uri) else {
            return Ok(None);
        };
        let findings_map = self.findings.read().await;
        let warn_lines: std::collections::HashSet<usize> = findings_map
            .get(uri)
            .map(|fs| {
                fs.iter()
                    .filter(|f| matches!(f.severity, Severity::Warning | Severity::Error))
                    .flat_map(|f| f.location.start_line..=f.location.end_line)
                    .collect()
            })
            .unwrap_or_default();
        let mut symbols: Vec<DocumentSymbol> = model
            .functions
            .iter()
            .map(|f| make_fn_symbol(f, &warn_lines))
            .collect();
        symbols.extend(
            model
                .classes
                .iter()
                .map(|c| make_class_symbol(c, &warn_lines)),
        );
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        let models = self.models.read().await;
        let Some(model) = models.get(uri) else {
            return Ok(None);
        };
        let findings_map = self.findings.read().await;
        let warn_lines: std::collections::HashSet<usize> = findings_map
            .get(uri)
            .map(|fs| {
                fs.iter()
                    .filter(|f| matches!(f.severity, Severity::Warning | Severity::Error))
                    .flat_map(|f| f.location.start_line..=f.location.end_line)
                    .collect()
            })
            .unwrap_or_default();
        let tokens = build_semantic_tokens(model, &warn_lines);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        let uri = &params.text_document.uri;
        let findings = self.findings.read().await;
        let diagnostics: Vec<Diagnostic> = findings
            .get(uri)
            .map(|fs| fs.iter().map(finding_to_diagnostic).collect())
            .unwrap_or_default();
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: diagnostics,
                },
            }),
        ))
    }

    async fn workspace_diagnostic(
        &self,
        _params: WorkspaceDiagnosticParams,
    ) -> Result<WorkspaceDiagnosticReportResult> {
        let findings_map = self.findings.read().await;
        let items: Vec<WorkspaceDocumentDiagnosticReport> = findings_map
            .iter()
            .filter_map(|(uri, fs)| {
                if fs.is_empty() {
                    return None;
                }
                Some(WorkspaceDocumentDiagnosticReport::Full(
                    WorkspaceFullDocumentDiagnosticReport {
                        uri: uri.clone(),
                        version: None,
                        full_document_diagnostic_report: FullDocumentDiagnosticReport {
                            result_id: None,
                            items: fs.iter().map(finding_to_diagnostic).collect(),
                        },
                    },
                ))
            })
            .collect();

        Ok(WorkspaceDiagnosticReportResult::Report(
            WorkspaceDiagnosticReport { items },
        ))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Build code actions from cha diagnostics.
fn collect_diagnostic_actions(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    diagnostics: &[Diagnostic],
    doc_text: Option<&String>,
) {
    for diag in diagnostics {
        if diag.source.as_deref() != Some("cha") {
            continue;
        }
        // Extract Method for long_method
        if let Some(text) = doc_text
            && diag.code == Some(NumberOrString::String("long_method".into()))
            && let Some(action) = build_extract_method(uri, &diag.range, text)
        {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }
        // Suggestion-based quick fixes
        if let Some(data) = &diag.data
            && let Ok(suggestions) = serde_json::from_value::<Vec<String>>(data.clone())
        {
            for suggestion in suggestions {
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: format!("Refactor: {}", suggestion),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diag.clone()]),
                    ..Default::default()
                }));
            }
        }
    }
}

/// Offer Extract Method for user selections spanning 3+ lines.
fn collect_selection_actions(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    range: &Range,
    doc_text: Option<&String>,
) {
    if let Some(text) = doc_text {
        let line_span = range.end.line.saturating_sub(range.start.line);
        if line_span >= 3
            && let Some(action) = build_extract_method(uri, range, text)
        {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }
    }
}

/// Build an Extract Method code action.
fn build_extract_method(uri: &Url, range: &Range, text: &str) -> Option<CodeAction> {
    let lines: Vec<&str> = text.lines().collect();
    let start = range.start.line as usize;
    let end = (range.end.line as usize).min(lines.len());
    if start >= end || start >= lines.len() {
        return None;
    }

    let selected = &lines[start..end];
    let edits = build_extract_edits(uri, range, selected, end);

    Some(CodeAction {
        title: "Extract Method".into(),
        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
        edit: Some(WorkspaceEdit {
            changes: Some(edits),
            ..Default::default()
        }),
        ..Default::default()
    })
}

fn build_extract_edits(
    uri: &Url,
    range: &Range,
    selected: &[&str],
    end: usize,
) -> HashMap<Url, Vec<TextEdit>> {
    let indent = selected
        .first()
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);
    let fn_name = "extracted";

    let body = selected
        .iter()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                format!("    {}", l.trim())
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let call = format!("{}{fn_name}();\n", " ".repeat(indent));
    let new_fn = format!("\nfn {fn_name}() {{\n{body}\n}}\n");
    let end_col = selected.last().map(|l| l.len() as u32).unwrap_or(0);

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![
            TextEdit {
                range: Range {
                    start: Position::new(range.start.line, 0),
                    end: Position::new(range.end.line, end_col),
                },
                new_text: call,
            },
            TextEdit {
                range: Range {
                    start: Position::new(end as u32, 0),
                    end: Position::new(end as u32, 0),
                },
                new_text: new_fn,
            },
        ],
    );
    changes
}

/// Entry point for the LSP server.
pub async fn run_lsp() {
    let cwd = std::env::current_dir().unwrap_or_default();
    let config = Config::load(&cwd);
    let registry = Arc::new(PluginRegistry::from_config(&config, &cwd));
    let plugin_dirs = vec![
        cwd.join(".cha/plugins"),
        dirs::home_dir().unwrap_or_default().join(".cha/plugins"),
    ];
    let eh = cha_core::env_hash(&cwd, &plugin_dirs);
    let cache = Arc::new(Mutex::new(ProjectCache::open(&cwd, eh)));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| ChaLsp {
        client,
        registry: registry.clone(),
        cwd: cwd.clone(),
        docs: Arc::new(RwLock::new(HashMap::new())),
        disabled_plugins: Arc::new(RwLock::new(Vec::new())),
        cache: cache.clone(),
        findings: Arc::new(RwLock::new(HashMap::new())),
        models: Arc::new(RwLock::new(HashMap::new())),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
