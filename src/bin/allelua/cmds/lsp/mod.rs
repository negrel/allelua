use std::sync::Arc;

use clap::{crate_name, crate_version};
use documents::Documents;
use tower_lsp::{
    jsonrpc::{Error as LspError, ErrorCode as LspErrorCode, Result as LspResult},
    lsp_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DocumentFormattingParams, InitializeParams, InitializeResult, InitializedParams,
        MessageType, OneOf, PositionEncodingKind, ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
        TextDocumentSyncSaveOptions, TextEdit,
    },
    Client, LanguageServer, LspService, Server,
};

mod documents;
mod format;

use format::*;

#[derive(Debug)]
struct Backend {
    client: Client,
    inner: Arc<tokio::sync::RwLock<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    documents: Documents,
}

macro_rules! lsp_log {
    ($self:ident, $lvl:tt, $fmt:literal $(, $arg:expr)*) => {
        $self.client.log_message(MessageType::$lvl, format!($fmt, $($arg),*)).await
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        let result = InitializeResult {
            server_info: Some(ServerInfo {
                name: crate_name!().to_owned(),
                version: Some(crate_version!().to_owned()),
            }),
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF8),
                document_formatting_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                    },
                )),
                ..Default::default()
            },
        };

        Ok(result)
    }

    async fn initialized(&self, _: InitializedParams) {
        lsp_log!(self, INFO, "server initialized!");
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        lsp_log!(self, LOG, "did_open {params:?}");
        let mut inner = self.inner.write().await;
        inner.documents.open(params.text_document);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        lsp_log!(self, LOG, "did_change {params:?}");

        let mut inner = self.inner.write().await;
        match inner
            .documents
            .change(params.text_document, params.content_changes)
        {
            Ok(()) => {}
            Err(err) => lsp_log!(self, ERROR, "failed to commit document change: {err}"),
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        lsp_log!(self, LOG, "did_close {params:?}");

        let mut inner = self.inner.write().await;
        inner.documents.close(params.text_document.uri);
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> LspResult<Option<Vec<TextEdit>>> {
        lsp_log!(self, LOG, "formatting {params:?}");

        let inner = self.inner.read().await;

        let uri = params.text_document.uri;
        let doc = match inner.documents.get(uri) {
            Some(doc) => doc,
            None => return Err(LspError::invalid_request()),
        };

        match format(&doc.text) {
            Ok(Some(edits)) => {
                for edit in edits.clone() {
                    let source = doc
                        .text
                        .to_owned()
                        .lines()
                        .enumerate()
                        .filter(|(i, _)| {
                            *i as u32 >= edit.range.start.line && (*i as u32) <= edit.range.end.line
                        })
                        .map(|(i, line)| {
                            if i as u32 == edit.range.start.line {
                                if i as u32 == edit.range.end.line {
                                    &line[edit.range.start.character as usize
                                        ..edit.range.end.character as usize]
                                } else {
                                    &line[edit.range.start.character as usize..line.len()]
                                }
                            } else if i as u32 == edit.range.end.line {
                                &line[0..edit.range.end.character as usize]
                            } else {
                                line
                            }
                        })
                        .collect::<Vec<&str>>()
                        .join("\n");

                    lsp_log!(self, INFO, "EDIT: {} -> {}", source, edit.new_text);
                }
                Ok(Some(edits))
            }
            Ok(None) => Ok(None),
            Err(err) => Err(LspError {
                code: LspErrorCode::ServerError(1),
                message: err.to_string().into(),
                data: None,
            }),
        }
    }

    async fn shutdown(&self) -> LspResult<()> {
        lsp_log!(self, LOG, "shutdown");

        Ok(())
    }
}

pub fn lsp() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        inner: Default::default(),
    });

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the tokio Runtime")
        .block_on(async {
            Server::new(stdin, stdout, socket).serve(service).await;
        });

    Ok(())
}
