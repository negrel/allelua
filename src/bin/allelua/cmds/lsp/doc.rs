use anyhow::bail;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range,
    TextDocumentContentChangeEvent, TextDocumentItem,
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
                            code_description: None,
                            source: Some("selene".to_string()),
                            message: diag.message,
                            related_information: None,
                            tags: None,
                            data: None,
                        }
                    })
                    .collect()
            }
            Err(err) => match err {
                full_moon::Error::AstError(err) => match err {
                    full_moon::ast::AstError::Empty => vec![],
                    full_moon::ast::AstError::NoEof => unreachable!(),
                    full_moon::ast::AstError::UnexpectedToken { token, additional } => {
                        let range = Range::new(
                            Position::new(
                                token.start_position().line() as u32,
                                token.start_position().character() as u32,
                            ),
                            Position::new(
                                token.end_position().line() as u32,
                                token.end_position().character() as u32,
                            ),
                        );

                        vec![Diagnostic {
                            range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("unexpected_token".to_string())),
                            code_description: None,
                            source: Some("full_moon".to_string()),
                            message: additional
                                .unwrap_or(std::borrow::Cow::Borrowed("Unexpected token."))
                                .to_string(),
                            related_information: None,
                            tags: None,
                            data: None,
                        }]
                    }
                },
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
                        full_moon::tokenizer::TokenizerErrorType::UnexpectedShebang => {
                            diag.message = "Unexpected shebang".to_owned()
                        }
                        full_moon::tokenizer::TokenizerErrorType::UnexpectedToken(char) => {
                            diag.message = format!("Unexpected token {char}")
                        }
                        full_moon::tokenizer::TokenizerErrorType::InvalidSymbol(symbol) => {
                            diag.message = format!("Invalid symbol {symbol}")
                        }
                    }

                    vec![diag]
                }
            },
        }
    }
}
