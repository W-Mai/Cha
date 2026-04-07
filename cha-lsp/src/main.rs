use std::path::PathBuf;
use std::sync::Arc;

use cha_core::{AnalysisContext, Config, Finding, PluginRegistry, Severity, SourceFile};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct ChaLsp {
    client: Client,
    registry: Arc<PluginRegistry>,
}

impl ChaLsp {
    fn analyze_and_publish(&self, uri: &Url, text: &str) {
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let file = SourceFile::new(path, text.to_string());
        let model = match cha_parser::parse_file(&file) {
            Some(m) => m,
            None => {
                let client = self.client.clone();
                let uri = uri.clone();
                tokio::spawn(async move {
                    client.publish_diagnostics(uri, vec![], None).await;
                });
                return;
            }
        };

        let ctx = AnalysisContext {
            file: &file,
            model: &model,
        };

        let mut findings = Vec::new();
        for plugin in self.registry.plugins() {
            findings.extend(plugin.analyze(&ctx));
        }

        let diagnostics = findings.iter().map(finding_to_diagnostic).collect();
        let client = self.client.clone();
        let uri = uri.clone();
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
        self.analyze_and_publish(&params.text_document.uri, &params.text_document.text);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.analyze_and_publish(&params.text_document.uri, &text);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.analyze_and_publish(&params.text_document.uri, &change.text);
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let mut actions = Vec::new();

        for diag in &params.context.diagnostics {
            if diag.source.as_deref() != Some("cha") {
                continue;
            }
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

#[tokio::main]
async fn main() {
    let cwd = std::env::current_dir().unwrap_or_default();
    let config = Config::load(&cwd);
    let registry = Arc::new(PluginRegistry::from_config(&config, &cwd));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| ChaLsp {
        client,
        registry: registry.clone(),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
