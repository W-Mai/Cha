use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use cha_core::{AnalysisContext, Config, Finding, PluginRegistry, Severity, SourceFile};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct ChaLsp {
    client: Client,
    registry: Arc<PluginRegistry>,
    docs: Arc<RwLock<HashMap<Url, String>>>,
}

impl ChaLsp {
    fn analyze_and_publish(&self, uri: &Url, text: &str) {
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let file = SourceFile::new(path, text.to_string());

        let diagnostics = self.collect_diagnostics(&file);
        self.publish(uri.clone(), diagnostics);
    }

    // Run all plugins on a single file and convert findings to diagnostics.
    fn collect_diagnostics(&self, file: &SourceFile) -> Vec<Diagnostic> {
        let model = match cha_parser::parse_file(file) {
            Some(m) => m,
            None => return vec![],
        };
        let ctx = AnalysisContext {
            file,
            model: &model,
        };
        self.registry
            .plugins()
            .iter()
            .flat_map(|p| p.analyze(&ctx))
            .map(|f| finding_to_diagnostic(&f))
            .collect()
    }

    // Spawn an async task to publish diagnostics to the client.
    fn publish(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        let client = self.client.clone();
        tokio::spawn(async move {
            client.publish_diagnostics(uri, diagnostics, None).await;
        });
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
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cha-lsp initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        self.docs.write().await.insert(uri.clone(), text.clone());
        self.analyze_and_publish(&uri, &text);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.docs
                .write()
                .await
                .insert(params.text_document.uri.clone(), text.clone());
            self.analyze_and_publish(&params.text_document.uri, &text);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.docs
                .write()
                .await
                .insert(params.text_document.uri.clone(), change.text.clone());
            self.analyze_and_publish(&params.text_document.uri, &change.text);
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

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| ChaLsp {
        client,
        registry: registry.clone(),
        docs: Arc::new(RwLock::new(HashMap::new())),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
