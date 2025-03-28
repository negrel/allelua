mod empty_if;

use std::fmt;

use codespan_reporting::diagnostic::Severity;
use full_moon::visitors::Visitor;

/// Rule defines a lint rule.
pub trait Rule: Visitor + fmt::Debug {
    fn diagnostics(&mut self) -> Vec<Diagnostic>;
}

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

/// Label define a message and a bytes range.
#[derive(Debug, PartialEq, Eq)]
pub struct Label {
    pub message: Option<String>,
    pub range: (u32, u32),
}
