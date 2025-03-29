use codespan_reporting::diagnostic::Severity;
use full_moon::{ast, node::Node, visitors::Visitor};

use super::{fullmoon_range_to_bytes_range, Diagnostic, Label, Rule};

#[derive(Debug, Default)]
pub struct EmptyIf {
    diags: Vec<Diagnostic>,
}

impl Rule for EmptyIf {
    fn diagnostics(&mut self) -> Vec<Diagnostic> {
        let mut diags = vec![];
        std::mem::swap(&mut self.diags, &mut diags);
        diags
    }
}

impl Visitor for EmptyIf {
    fn visit_if(&mut self, if_block: &ast::If) {
        if block_is_empty(if_block.block()) {
            self.diags.push(Diagnostic {
                severity: Severity::Note,
                code: "emtpy_if",
                message: "empty if block".to_owned(),
                notes: vec![],
                primary_label: Label {
                    message: Some("block is empty".to_string()),
                    range: if_block
                        .range()
                        .map(|(start, end)| (start.bytes() as u32, end.bytes() as u32))
                        .unwrap(),
                },
                secondary_labels: vec![],
            })
        }

        if let Some(else_ifs) = if_block.else_if() {
            for else_if in else_ifs {
                if block_is_empty(else_if.block()) {
                    self.diags.push(Diagnostic {
                        severity: Severity::Note,
                        code: "emtpy_if",
                        message: "empty else if block".to_owned(),
                        notes: vec![],
                        primary_label: Label {
                            message: Some("block is empty".to_string()),
                            range: if_block
                                .range()
                                .map(|(start, end)| (start.bytes() as u32, end.bytes() as u32))
                                .unwrap(),
                        },
                        secondary_labels: vec![],
                    })
                }
            }
        }

        if let Some(else_block) = if_block.else_block() {
            if block_is_empty(else_block) {
                self.diags.push(Diagnostic {
                    severity: Severity::Note,
                    code: "emtpy_if",
                    message: "empty else block".to_owned(),
                    notes: vec![],
                    primary_label: Label {
                        message: Some("block is empty".to_string()),
                        range: if_block.range().map(fullmoon_range_to_bytes_range).unwrap(),
                    },
                    secondary_labels: vec![],
                })
            }
        }
    }
}

fn block_is_empty(block: &ast::Block) -> bool {
    block.last_stmt().is_none() && block.stmts().next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_if() {
        let ast = full_moon::parse(
            r#"
if 1 == 1 then end
"#,
        )
        .unwrap();

        let mut rule = EmptyIf::default();
        rule.visit_ast(&ast);

        assert!(rule.diagnostics().len() == 1);
    }

    #[test]
    fn non_empty_if() {
        let ast = full_moon::parse(
            r#"
if foo then
    print("foo")
end
"#,
        )
        .unwrap();

        let mut rule = EmptyIf::default();
        rule.visit_ast(&ast);

        assert!(rule.diagnostics().is_empty());
    }

    #[test]
    fn empty_else_if() {
        let ast = full_moon::parse(
            r#"
if foo then
    print("foo")
elseif bar then
elseif baz then end
"#,
        )
        .unwrap();

        let mut rule = EmptyIf::default();
        rule.visit_ast(&ast);

        assert!(rule.diagnostics().len() == 2);
    }

    #[test]
    fn empty_else() {
        let ast = full_moon::parse(
            r#"
if foo then
    print("foo")
else end
"#,
        )
        .unwrap();

        let mut rule = EmptyIf::default();
        rule.visit_ast(&ast);

        assert!(rule.diagnostics().len() == 1);
    }
}
