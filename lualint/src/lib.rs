mod rules;

use full_moon::{
    ast::{
        lua52::{Goto, Label},
        span::ContainedSpan,
        Assignment, Ast, Block, Call, Do, ElseIf, Expression, Field, FunctionArgs, FunctionBody,
        FunctionCall, FunctionDeclaration, FunctionName, GenericFor, If, Index, LastStmt,
        LocalAssignment, LocalFunction, MethodCall, NumericFor, Parameter, Prefix, Repeat, Return,
        Stmt, Suffix, TableConstructor, UnOp, Var, VarExpression, While,
    },
    tokenizer::{Token, TokenReference},
    visitors::Visitor,
};
use rules::{Diagnostic, Rule};

#[derive(Debug, Default)]
pub struct Linter {
    rules: Vec<Box<dyn Rule>>,
}

impl Linter {
    /// Creates a new [Linter] with all rules.
    pub fn new() -> Self {
        Self::with_rules(vec![])
    }

    /// Creates a new linter with provided [Rule].
    pub fn with_rules(rules: Vec<Box<dyn Rule>>) -> Self {
        Self { rules }
    }

    /// Lint provided AST and returns diagnostics of all rules.
    pub fn lint(&mut self, ast: &Ast) -> Vec<Diagnostic> {
        self.visit_ast(ast);
        let mut diags = Vec::new();

        for r in self.rules.iter_mut() {
            diags.extend(r.diagnostics())
        }

        diags
    }
}

macro_rules! forward_visit {
    ($self:ident, $n:ident, $node:ident) => {
        for r in $self.rules.iter_mut() {
            r.$n($node);
        }
    };
}

impl Visitor for Linter {
    fn visit_anonymous_call(&mut self, node: &FunctionArgs) {
        forward_visit!(self, visit_anonymous_call, node);
    }

    fn visit_anonymous_call_end(&mut self, node: &FunctionArgs) {
        forward_visit!(self, visit_anonymous_call_end, node);
    }

    fn visit_assignment(&mut self, node: &Assignment) {
        forward_visit!(self, visit_assignment, node);
    }

    fn visit_assignment_end(&mut self, node: &Assignment) {
        forward_visit!(self, visit_assignment_end, node);
    }

    fn visit_block(&mut self, node: &Block) {
        forward_visit!(self, visit_block, node);
    }

    fn visit_block_end(&mut self, node: &Block) {
        forward_visit!(self, visit_block_end, node);
    }

    fn visit_call(&mut self, node: &Call) {
        forward_visit!(self, visit_call, node);
    }

    fn visit_call_end(&mut self, node: &Call) {
        forward_visit!(self, visit_call_end, node);
    }

    fn visit_contained_span(&mut self, node: &ContainedSpan) {
        forward_visit!(self, visit_contained_span, node);
    }

    fn visit_contained_span_end(&mut self, node: &ContainedSpan) {
        forward_visit!(self, visit_contained_span_end, node);
    }

    fn visit_do(&mut self, node: &Do) {
        forward_visit!(self, visit_do, node);
    }

    fn visit_do_end(&mut self, node: &Do) {
        forward_visit!(self, visit_do_end, node);
    }

    fn visit_else_if(&mut self, node: &ElseIf) {
        forward_visit!(self, visit_else_if, node);
    }

    fn visit_else_if_end(&mut self, node: &ElseIf) {
        forward_visit!(self, visit_else_if_end, node);
    }

    fn visit_eof(&mut self, node: &TokenReference) {
        forward_visit!(self, visit_eof, node);
    }

    fn visit_eof_end(&mut self, node: &TokenReference) {
        forward_visit!(self, visit_eof_end, node);
    }

    fn visit_expression(&mut self, node: &Expression) {
        forward_visit!(self, visit_expression, node);
    }

    fn visit_expression_end(&mut self, node: &Expression) {
        forward_visit!(self, visit_expression_end, node);
    }

    fn visit_field(&mut self, node: &Field) {
        forward_visit!(self, visit_field, node);
    }

    fn visit_field_end(&mut self, node: &Field) {
        forward_visit!(self, visit_field_end, node);
    }

    fn visit_function_args(&mut self, node: &FunctionArgs) {
        forward_visit!(self, visit_function_args, node);
    }

    fn visit_function_args_end(&mut self, node: &FunctionArgs) {
        forward_visit!(self, visit_function_args_end, node);
    }

    fn visit_function_body(&mut self, node: &FunctionBody) {
        forward_visit!(self, visit_function_body, node);
    }

    fn visit_function_body_end(&mut self, node: &FunctionBody) {
        forward_visit!(self, visit_function_body_end, node);
    }

    fn visit_function_call(&mut self, node: &FunctionCall) {
        forward_visit!(self, visit_function_call, node);
    }

    fn visit_function_call_end(&mut self, node: &FunctionCall) {
        forward_visit!(self, visit_function_call_end, node);
    }

    fn visit_function_declaration(&mut self, node: &FunctionDeclaration) {
        forward_visit!(self, visit_function_declaration, node);
    }

    fn visit_function_declaration_end(&mut self, node: &FunctionDeclaration) {
        forward_visit!(self, visit_function_declaration_end, node);
    }

    fn visit_function_name(&mut self, node: &FunctionName) {
        forward_visit!(self, visit_function_name, node);
    }

    fn visit_function_name_end(&mut self, node: &FunctionName) {
        forward_visit!(self, visit_function_name_end, node);
    }

    fn visit_generic_for(&mut self, node: &GenericFor) {
        forward_visit!(self, visit_generic_for, node);
    }

    fn visit_generic_for_end(&mut self, node: &GenericFor) {
        forward_visit!(self, visit_generic_for_end, node);
    }

    fn visit_if(&mut self, node: &If) {
        forward_visit!(self, visit_if, node);
    }

    fn visit_if_end(&mut self, node: &If) {
        forward_visit!(self, visit_if_end, node);
    }

    fn visit_index(&mut self, node: &Index) {
        forward_visit!(self, visit_index, node);
    }

    fn visit_index_end(&mut self, node: &Index) {
        forward_visit!(self, visit_index_end, node);
    }

    fn visit_local_assignment(&mut self, node: &LocalAssignment) {
        forward_visit!(self, visit_local_assignment, node);
    }

    fn visit_local_assignment_end(&mut self, node: &LocalAssignment) {
        forward_visit!(self, visit_local_assignment_end, node);
    }

    fn visit_local_function(&mut self, node: &LocalFunction) {
        forward_visit!(self, visit_local_function, node);
    }

    fn visit_local_function_end(&mut self, node: &LocalFunction) {
        forward_visit!(self, visit_local_function_end, node);
    }

    fn visit_last_stmt(&mut self, node: &LastStmt) {
        forward_visit!(self, visit_last_stmt, node);
    }

    fn visit_last_stmt_end(&mut self, node: &LastStmt) {
        forward_visit!(self, visit_last_stmt_end, node);
    }

    fn visit_method_call(&mut self, node: &MethodCall) {
        forward_visit!(self, visit_method_call, node);
    }

    fn visit_method_call_end(&mut self, node: &MethodCall) {
        forward_visit!(self, visit_method_call_end, node);
    }

    fn visit_numeric_for(&mut self, node: &NumericFor) {
        forward_visit!(self, visit_numeric_for, node);
    }

    fn visit_numeric_for_end(&mut self, node: &NumericFor) {
        forward_visit!(self, visit_numeric_for_end, node);
    }

    fn visit_parameter(&mut self, node: &Parameter) {
        forward_visit!(self, visit_parameter, node);
    }

    fn visit_parameter_end(&mut self, node: &Parameter) {
        forward_visit!(self, visit_parameter_end, node);
    }

    fn visit_prefix(&mut self, node: &Prefix) {
        forward_visit!(self, visit_prefix, node);
    }

    fn visit_prefix_end(&mut self, node: &Prefix) {
        forward_visit!(self, visit_prefix_end, node);
    }

    fn visit_return(&mut self, node: &Return) {
        forward_visit!(self, visit_return, node);
    }

    fn visit_return_end(&mut self, node: &Return) {
        forward_visit!(self, visit_return_end, node);
    }

    fn visit_repeat(&mut self, node: &Repeat) {
        forward_visit!(self, visit_repeat, node);
    }

    fn visit_repeat_end(&mut self, node: &Repeat) {
        forward_visit!(self, visit_repeat_end, node);
    }

    fn visit_stmt(&mut self, node: &Stmt) {
        forward_visit!(self, visit_stmt, node);
    }

    fn visit_stmt_end(&mut self, node: &Stmt) {
        forward_visit!(self, visit_stmt_end, node);
    }

    fn visit_suffix(&mut self, node: &Suffix) {
        forward_visit!(self, visit_suffix, node);
    }

    fn visit_suffix_end(&mut self, node: &Suffix) {
        forward_visit!(self, visit_suffix_end, node);
    }

    fn visit_table_constructor(&mut self, node: &TableConstructor) {
        forward_visit!(self, visit_table_constructor, node);
    }

    fn visit_table_constructor_end(&mut self, node: &TableConstructor) {
        forward_visit!(self, visit_table_constructor_end, node);
    }

    fn visit_token_reference(&mut self, node: &TokenReference) {
        forward_visit!(self, visit_token_reference, node);
    }

    fn visit_token_reference_end(&mut self, node: &TokenReference) {
        forward_visit!(self, visit_token_reference_end, node);
    }

    fn visit_un_op(&mut self, node: &UnOp) {
        forward_visit!(self, visit_un_op, node);
    }

    fn visit_un_op_end(&mut self, node: &UnOp) {
        forward_visit!(self, visit_un_op_end, node);
    }

    fn visit_var(&mut self, node: &Var) {
        forward_visit!(self, visit_var, node);
    }

    fn visit_var_end(&mut self, node: &Var) {
        forward_visit!(self, visit_var_end, node);
    }

    fn visit_var_expression(&mut self, node: &VarExpression) {
        forward_visit!(self, visit_var_expression, node);
    }

    fn visit_var_expression_end(&mut self, node: &VarExpression) {
        forward_visit!(self, visit_var_expression_end, node);
    }

    fn visit_while(&mut self, node: &While) {
        forward_visit!(self, visit_while, node);
    }

    fn visit_while_end(&mut self, node: &While) {
        forward_visit!(self, visit_while_end, node);
    }

    fn visit_goto(&mut self, node: &Goto) {
        forward_visit!(self, visit_goto, node);
    }

    fn visit_goto_end(&mut self, node: &Goto) {
        forward_visit!(self, visit_goto_end, node);
    }

    fn visit_label(&mut self, node: &Label) {
        forward_visit!(self, visit_label, node);
    }

    fn visit_label_end(&mut self, node: &Label) {
        forward_visit!(self, visit_label_end, node);
    }

    fn visit_identifier(&mut self, token: &Token) {
        forward_visit!(self, visit_identifier, token);
    }

    fn visit_multi_line_comment(&mut self, token: &Token) {
        forward_visit!(self, visit_multi_line_comment, token);
    }

    fn visit_number(&mut self, token: &Token) {
        forward_visit!(self, visit_number, token);
    }

    fn visit_single_line_comment(&mut self, token: &Token) {
        forward_visit!(self, visit_single_line_comment, token);
    }

    fn visit_string_literal(&mut self, token: &Token) {
        forward_visit!(self, visit_string_literal, token);
    }

    fn visit_symbol(&mut self, token: &Token) {
        forward_visit!(self, visit_symbol, token);
    }

    fn visit_token(&mut self, token: &Token) {
        forward_visit!(self, visit_token, token);
    }

    fn visit_whitespace(&mut self, token: &Token) {
        forward_visit!(self, visit_whitespace, token);
    }
}

#[cfg(test)]
mod test {
    use codespan_reporting::diagnostic::Severity;

    use super::*;

    #[test]
    pub fn forward_visit() {
        #[derive(Debug, Default)]
        struct MyRule {
            diags: Vec<Diagnostic>,
        }

        impl Visitor for MyRule {
            fn visit_do(&mut self, _node: &Do) {
                self.diags.push(Diagnostic {
                    severity: Severity::Error,
                    code: "foo",
                    message: "foo".to_owned(),
                    notes: vec![],
                    primary_label: rules::Label {
                        message: None,
                        range: (0, 0),
                    },
                    secondary_labels: vec![],
                })
            }
        }

        impl Rule for MyRule {
            fn diagnostics(&mut self) -> Vec<rules::Diagnostic> {
                let mut diags = vec![];
                std::mem::swap(&mut self.diags, &mut diags);
                diags
            }
        }

        let mut linter = Linter::with_rules(vec![
            Box::new(MyRule::default()),
            Box::new(MyRule::default()),
        ]);

        let ast = full_moon::parse(
            r#"
do
    local foo = "bar"
    do
        local foo = "baz"
    end
end
"#,
        )
        .unwrap();

        let diags = linter.lint(&ast);
        assert!(diags.len() == 4);
    }
}
