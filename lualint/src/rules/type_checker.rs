use full_moon::{
    ast::{
        lua52::{Goto, Label},
        span::ContainedSpan,
        Assignment, Block, Call, Do, ElseIf, Expression, Field, FunctionArgs, FunctionBody,
        FunctionCall, FunctionDeclaration, FunctionName, GenericFor, If, Index, LastStmt,
        LocalAssignment, LocalFunction, MethodCall, NumericFor, Parameter, Prefix, Repeat, Return,
        Stmt, Suffix, TableConstructor, UnOp, Var, VarExpression, While,
    },
    tokenizer::{Token, TokenReference},
    visitors::Visitor,
};

use super::{Diagnostic, Rule};

#[derive(Debug, Default)]
pub struct TypeChecker {
    checker: luatypes::Checker,
}

impl Rule for TypeChecker {
    fn diagnostics(&mut self) -> Vec<Diagnostic> {
        self.checker
            .errors()
            .into_iter()
            .map(|err| Diagnostic {
                severity: codespan_reporting::diagnostic::Severity::Error,
                code: "type_error",
                message: err.to_string(),
                notes: vec![],
                primary_label: super::Label {
                    message: None,
                    range: err.range,
                },
                secondary_labels: vec![],
            })
            .collect()
    }
}

impl Visitor for TypeChecker {
    fn visit_anonymous_call(&mut self, node: &FunctionArgs) {
        self.checker.visit_anonymous_call(node);
    }

    fn visit_anonymous_call_end(&mut self, node: &FunctionArgs) {
        self.checker.visit_anonymous_call_end(node);
    }

    fn visit_assignment(&mut self, node: &Assignment) {
        self.checker.visit_assignment(node);
    }

    fn visit_assignment_end(&mut self, node: &Assignment) {
        self.checker.visit_assignment_end(node);
    }

    fn visit_block(&mut self, node: &Block) {
        self.checker.visit_block(node);
    }

    fn visit_block_end(&mut self, node: &Block) {
        self.checker.visit_block_end(node);
    }

    fn visit_call(&mut self, node: &Call) {
        self.checker.visit_call(node);
    }

    fn visit_call_end(&mut self, node: &Call) {
        self.checker.visit_call_end(node);
    }

    fn visit_contained_span(&mut self, node: &ContainedSpan) {
        self.checker.visit_contained_span(node);
    }

    fn visit_contained_span_end(&mut self, node: &ContainedSpan) {
        self.checker.visit_contained_span_end(node);
    }

    fn visit_do(&mut self, node: &Do) {
        self.checker.visit_do(node);
    }

    fn visit_do_end(&mut self, node: &Do) {
        self.checker.visit_do_end(node);
    }

    fn visit_else_if(&mut self, node: &ElseIf) {
        self.checker.visit_else_if(node);
    }

    fn visit_else_if_end(&mut self, node: &ElseIf) {
        self.checker.visit_else_if_end(node);
    }

    fn visit_eof(&mut self, node: &TokenReference) {
        self.checker.visit_eof(node);
    }

    fn visit_eof_end(&mut self, node: &TokenReference) {
        self.checker.visit_eof_end(node)
    }

    fn visit_expression(&mut self, node: &Expression) {
        self.checker.visit_expression(node);
    }

    fn visit_expression_end(&mut self, node: &Expression) {
        self.checker.visit_expression_end(node);
    }

    fn visit_field(&mut self, node: &Field) {
        self.checker.visit_field(node);
    }

    fn visit_field_end(&mut self, node: &Field) {
        self.checker.visit_field_end(node);
    }

    fn visit_function_args(&mut self, node: &FunctionArgs) {
        self.checker.visit_function_args(node);
    }

    fn visit_function_args_end(&mut self, node: &FunctionArgs) {
        self.checker.visit_function_args_end(node);
    }

    fn visit_function_body(&mut self, node: &FunctionBody) {
        self.checker.visit_function_body(node);
    }

    fn visit_function_body_end(&mut self, node: &FunctionBody) {
        self.checker.visit_function_body_end(node);
    }

    fn visit_function_call(&mut self, node: &FunctionCall) {
        self.checker.visit_function_call(node);
    }

    fn visit_function_call_end(&mut self, node: &FunctionCall) {
        self.checker.visit_function_call_end(node);
    }

    fn visit_function_declaration(&mut self, node: &FunctionDeclaration) {
        self.checker.visit_function_declaration(node);
    }

    fn visit_function_declaration_end(&mut self, node: &FunctionDeclaration) {
        self.checker.visit_function_declaration_end(node);
    }

    fn visit_function_name(&mut self, node: &FunctionName) {
        self.checker.visit_function_name(node);
    }

    fn visit_function_name_end(&mut self, node: &FunctionName) {
        self.checker.visit_function_name_end(node);
    }

    fn visit_generic_for(&mut self, node: &GenericFor) {
        self.checker.visit_generic_for(node);
    }

    fn visit_generic_for_end(&mut self, node: &GenericFor) {
        self.checker.visit_generic_for_end(node);
    }

    fn visit_if(&mut self, node: &If) {
        self.checker.visit_if(node);
    }

    fn visit_if_end(&mut self, node: &If) {
        self.checker.visit_if_end(node);
    }

    fn visit_index(&mut self, node: &Index) {
        self.checker.visit_index(node);
    }

    fn visit_index_end(&mut self, node: &Index) {
        self.checker.visit_index_end(node);
    }

    fn visit_local_assignment(&mut self, node: &LocalAssignment) {
        self.checker.visit_local_assignment(node);
    }

    fn visit_local_assignment_end(&mut self, node: &LocalAssignment) {
        self.checker.visit_local_assignment_end(node);
    }

    fn visit_local_function(&mut self, node: &LocalFunction) {
        self.checker.visit_local_function(node);
    }

    fn visit_local_function_end(&mut self, node: &LocalFunction) {
        self.checker.visit_local_function_end(node);
    }

    fn visit_last_stmt(&mut self, node: &LastStmt) {
        self.checker.visit_last_stmt(node);
    }

    fn visit_last_stmt_end(&mut self, node: &LastStmt) {
        self.checker.visit_last_stmt_end(node);
    }

    fn visit_method_call(&mut self, node: &MethodCall) {
        self.checker.visit_method_call(node);
    }

    fn visit_method_call_end(&mut self, node: &MethodCall) {
        self.checker.visit_method_call_end(node);
    }

    fn visit_numeric_for(&mut self, node: &NumericFor) {
        self.checker.visit_numeric_for(node);
    }

    fn visit_numeric_for_end(&mut self, node: &NumericFor) {
        self.checker.visit_numeric_for_end(node);
    }

    fn visit_parameter(&mut self, node: &Parameter) {
        self.checker.visit_parameter(node);
    }

    fn visit_parameter_end(&mut self, node: &Parameter) {
        self.checker.visit_parameter_end(node);
    }

    fn visit_prefix(&mut self, node: &Prefix) {
        self.checker.visit_prefix(node);
    }

    fn visit_prefix_end(&mut self, node: &Prefix) {
        self.checker.visit_prefix_end(node);
    }

    fn visit_return(&mut self, node: &Return) {
        self.checker.visit_return(node);
    }

    fn visit_return_end(&mut self, node: &Return) {
        self.checker.visit_return_end(node);
    }

    fn visit_repeat(&mut self, node: &Repeat) {
        self.checker.visit_repeat(node);
    }

    fn visit_repeat_end(&mut self, node: &Repeat) {
        self.checker.visit_repeat_end(node);
    }

    fn visit_stmt(&mut self, node: &Stmt) {
        self.checker.visit_stmt(node);
    }

    fn visit_stmt_end(&mut self, node: &Stmt) {
        self.checker.visit_stmt_end(node);
    }

    fn visit_suffix(&mut self, node: &Suffix) {
        self.checker.visit_suffix(node);
    }

    fn visit_suffix_end(&mut self, node: &Suffix) {
        self.checker.visit_suffix_end(node);
    }

    fn visit_table_constructor(&mut self, node: &TableConstructor) {
        self.checker.visit_table_constructor(node);
    }

    fn visit_table_constructor_end(&mut self, node: &TableConstructor) {
        self.checker.visit_table_constructor_end(node);
    }

    fn visit_token_reference(&mut self, node: &TokenReference) {
        self.checker.visit_token_reference(node);
    }

    fn visit_token_reference_end(&mut self, node: &TokenReference) {
        self.checker.visit_token_reference_end(node);
    }

    fn visit_un_op(&mut self, node: &UnOp) {
        self.checker.visit_un_op(node);
    }

    fn visit_un_op_end(&mut self, node: &UnOp) {
        self.checker.visit_un_op_end(node);
    }

    fn visit_var(&mut self, node: &Var) {
        self.checker.visit_var(node);
    }

    fn visit_var_end(&mut self, node: &Var) {
        self.checker.visit_var_end(node);
    }

    fn visit_var_expression(&mut self, node: &VarExpression) {
        self.checker.visit_var_expression(node);
    }

    fn visit_var_expression_end(&mut self, node: &VarExpression) {
        self.checker.visit_var_expression_end(node);
    }

    fn visit_while(&mut self, node: &While) {
        self.checker.visit_while(node);
    }

    fn visit_while_end(&mut self, node: &While) {
        self.checker.visit_while_end(node);
    }

    fn visit_goto(&mut self, node: &Goto) {
        self.checker.visit_goto(node);
    }

    fn visit_goto_end(&mut self, node: &Goto) {
        self.checker.visit_goto_end(node);
    }

    fn visit_label(&mut self, node: &Label) {
        self.checker.visit_label(node);
    }

    fn visit_label_end(&mut self, node: &Label) {
        self.checker.visit_label_end(node);
    }

    fn visit_identifier(&mut self, node: &Token) {
        self.checker.visit_identifier(node);
    }

    fn visit_multi_line_comment(&mut self, node: &Token) {
        self.checker.visit_multi_line_comment(node);
    }

    fn visit_number(&mut self, node: &Token) {
        self.checker.visit_number(node);
    }

    fn visit_single_line_comment(&mut self, node: &Token) {
        self.checker.visit_single_line_comment(node);
    }

    fn visit_string_literal(&mut self, node: &Token) {
        self.checker.visit_string_literal(node);
    }

    fn visit_symbol(&mut self, node: &Token) {
        self.checker.visit_symbol(node);
    }

    fn visit_token(&mut self, node: &Token) {
        self.checker.visit_token(node);
    }

    fn visit_whitespace(&mut self, node: &Token) {
        self.checker.visit_whitespace(node);
    }
}
