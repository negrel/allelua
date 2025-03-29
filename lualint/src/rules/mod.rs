use std::fmt;

use codespan::FileId;
use codespan_reporting::diagnostic::Severity;
use full_moon::{tokenizer::Position, visitors::Visitor};

/// Rule defines a lint rule.
pub trait Rule: Visitor + fmt::Debug {
    fn diagnostics(&mut self) -> Vec<Diagnostic>;
}

mod empty_if;
mod type_checker;

pub use empty_if::*;
pub use type_checker::*;

/// Diagnostic defines a lint rule diagnostic.
#[derive(Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub notes: Vec<String>,
    pub primary_label: Label,
    pub secondary_labels: Vec<Label>,
}

impl Diagnostic {
    pub fn into_codespan_diagnostic(
        self,
        file_id: FileId,
    ) -> codespan_reporting::diagnostic::Diagnostic<FileId> {
        let mut labels = Vec::with_capacity(1 + self.secondary_labels.len());
        labels.push(self.primary_label.codespan_label(file_id));
        labels.extend(&mut self.secondary_labels.iter().map(|label| {
            codespan_reporting::diagnostic::Label::secondary(
                file_id,
                codespan::Span::new(label.range.0, label.range.1),
            )
            .with_message(label.message.as_ref().unwrap_or(&"".to_owned()).to_owned())
        }));

        codespan_reporting::diagnostic::Diagnostic {
            code: Some(self.code.to_owned()),
            labels,
            message: self.message.to_owned(),
            notes: self.notes,
            severity: self.severity,
        }
    }
}

/// Label define a message and a bytes range.
#[derive(Debug, PartialEq, Eq)]
pub struct Label {
    pub message: Option<String>,
    pub range: (u32, u32),
}

impl Label {
    pub fn codespan_label(
        &self,
        file_id: codespan::FileId,
    ) -> codespan_reporting::diagnostic::Label<codespan::FileId> {
        codespan_reporting::diagnostic::Label::primary(
            file_id.to_owned(),
            codespan::Span::new(self.range.0, self.range.1),
        )
        .with_message(self.message.as_ref().unwrap_or(&"".to_owned()).to_owned())
    }
}

fn fullmoon_range_to_bytes_range((start, end): (Position, Position)) -> (u32, u32) {
    (start.bytes() as u32, end.bytes() as u32)
}
