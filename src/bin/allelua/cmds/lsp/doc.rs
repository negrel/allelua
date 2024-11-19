use anyhow::bail;
use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range,
    TextDocumentContentChangeEvent, TextDocumentItem, Url,
};

use crate::cmds::lint_checker;

use super::byte_index_to_position;

#[derive(Debug)]
pub struct Doc {
    pub item: TextDocumentItem,
}

impl From<TextDocumentItem> for Doc {
    fn from(value: TextDocumentItem) -> Self {
        Self { item: value }
    }
}

impl Doc {
    pub fn change(
        &mut self,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> anyhow::Result<()> {
        let doc = &mut self.item;

        if changes.len() != 1 {
            bail!("only full TextDocumentSyncKind is supported")
        }

        if version <= doc.version {
            bail!("changes is older than stored document")
        }

        doc.version = version;
        doc.text = changes[0].text.clone();
        Ok(())
    }

    pub fn diagnostic(&self) -> Vec<Diagnostic> {
        match full_moon::parse(&self.item.text) {
            Ok(ast) => {
                let checker = lint_checker();
                let diagnostics = checker.test_on(&ast);
                diagnostics
                    .into_iter()
                    .map(|diag| {
                        let severity = diag.severity;
                        let diag = diag.diagnostic;

                        let range = diag.primary_label.range;
                        let range = Range::new(
                            byte_index_to_position(&self.item.text, range.0 as usize),
                            byte_index_to_position(&self.item.text, range.1 as usize),
                        );
                        let code_desc_url = format!(
                            "https://kampfkarren.github.io/selene/lints/{}.html",
                            diag.code
                        );

                        Diagnostic {
                            range,
                            severity: match severity {
                                selene_lib::lints::Severity::Allow => {
                                    Some(DiagnosticSeverity::HINT)
                                }
                                selene_lib::lints::Severity::Error => {
                                    Some(DiagnosticSeverity::ERROR)
                                }
                                selene_lib::lints::Severity::Warning => {
                                    Some(DiagnosticSeverity::WARNING)
                                }
                            },
                            code: Some(NumberOrString::String(diag.code.to_owned())),
                            code_description: Some(CodeDescription {
                                href: Url::parse(&code_desc_url).unwrap(),
                            }),
                            source: Some("selene".to_string()),
                            message: diag.message,
                            related_information: None,
                            tags: None,
                            data: None,
                        }
                    })
                    .collect()
            }
            Err(errors) => errors
                .iter()
                .flat_map(|err| match err {
                    full_moon::Error::AstError(err) => {
                        let (start_position, end_position) = err.range();
                        let range = Range::new(
                            Position::new(
                                start_position.line() as u32,
                                start_position.character() as u32,
                            ),
                            Position::new(
                                end_position.line() as u32,
                                end_position.character() as u32,
                            ),
                        );

                        Some(Diagnostic {
                            range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("unexpected_token".to_string())),
                            code_description: None,
                            source: Some("full_moon".to_string()),
                            message: err.error_message().to_owned(),
                            related_information: None,
                            tags: None,
                            data: None,
                        })
                    }
                    full_moon::Error::TokenizerError(err) => {
                        let pos = Position::new(
                            // LSP clients expect zero based position.
                            (err.position().line() - 1) as u32,
                            (err.position().character() - 1) as u32,
                        );
                        let range = Range::new(pos, pos);
                        let mut diag = Diagnostic {
                            range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            source: Some("full_moon".to_owned()),
                            ..Default::default()
                        };

                        match err.error() {
                            full_moon::tokenizer::TokenizerErrorType::UnclosedComment => {
                                diag.message = "Unclosed comment".to_owned()
                            }
                            full_moon::tokenizer::TokenizerErrorType::UnclosedString => {
                                diag.message = "Unclosed string".to_owned()
                            }
                            full_moon::tokenizer::TokenizerErrorType::UnexpectedToken(char) => {
                                diag.message = format!("Unexpected token {char}")
                            }
                            full_moon::tokenizer::TokenizerErrorType::InvalidSymbol(symbol) => {
                                diag.message = format!("Invalid symbol {symbol}")
                            }
                            full_moon::tokenizer::TokenizerErrorType::InvalidNumber => {
                                diag.message = format!("invalid number")
                            }
                        }

                        Some(diag)
                    }
                })
                .collect(),
        }
    }
}
