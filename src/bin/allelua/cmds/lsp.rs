use std::fs;

use anyhow::Context;
use clap::{crate_name, crate_version};
use tower_lsp::{
    jsonrpc,
    lsp_types::{
        DocumentFormattingParams, InitializeParams, InitializeResult, InitializedParams,
        MessageType, OneOf, Position, Range, ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
        TextDocumentSyncSaveOptions, TextEdit,
    },
    Client, LanguageServer, LspService, Server,
};

use crate::cmds::fmt;

type TowerResult<T> = tower_lsp::jsonrpc::Result<T>;

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> TowerResult<InitializeResult> {
        let result = InitializeResult {
            server_info: Some(ServerInfo {
                name: crate_name!().to_owned(),
                version: Some(crate_version!().to_owned()),
            }),
            capabilities: ServerCapabilities {
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
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> TowerResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        eprintln!("formatting {uri:?}");

        let result = tokio::task::spawn_blocking(move || {
            let fpath = uri.path();
            let source = fs::read_to_string(fpath)
                .with_context(|| format!("failed to read lua file {fpath:?}"))?;
            // Format.
            let formatted_source = fmt::format_str(&source)
                .with_context(|| format!("failed to format lua file {fpath:?}"))?;

            // TODO: convert similar::TextDiff to Vec<TextEdit>.
            let mut edits = vec![
                // Delete source.
                TextEdit {
                    range: Range {
                        start: Position::new(0, 0),
                        end: Position::new(source.lines().count() as u32, u32::MAX),
                    },
                    new_text: "".to_owned(),
                },
                // Insert formatted_source.
                TextEdit {
                    range: Range {
                        start: Position::new(0, 0),
                        end: Position::new(0, 0),
                    },
                    new_text: formatted_source,
                },
            ];
            edits.reverse();

            Ok::<_, anyhow::Error>(edits)
        })
        .await
        .unwrap();

        match result {
            Ok(edits) => Ok(Some(edits)),
            Err(err) => Err(jsonrpc::Error {
                code: jsonrpc::ErrorCode::ServerError(1),
                message: err.to_string().into(),
                data: None,
            }),
        }
    }

    async fn shutdown(&self) -> TowerResult<()> {
        Ok(())
    }
}

pub fn lsp() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the tokio Runtime")
        .block_on(async {
            Server::new(stdin, stdout, socket).serve(service).await;
        });

    Ok(())
}
