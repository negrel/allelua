use core::fmt;

use full_moon::{
    ast::{Ast, Expression, LastStmt},
    node::Node,
    visitors::Visitor,
};
use holylib::types::{Type, VM};

/// Checker define a Lua type checker.
#[derive(Debug)]
pub struct Checker {
    vm: VM,
    errors: Vec<Error>,
}

impl Checker {
    pub fn new() -> Self {
        Self {
            vm: VM::new().unwrap(),
            errors: Vec::new(),
        }
    }

    pub fn check(&mut self, ast: &Ast) -> Result<(), Vec<Error>> {
        self.visit_ast(ast);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }

    pub fn errors(&mut self) -> Vec<Error> {
        let mut errs = vec![];
        std::mem::swap(&mut self.errors, &mut errs);
        errs
    }

    fn type_of(&self, expr: &Expression) -> Result<(Type, Option<mlua::Value>), String> {
        let (t, literal): (Type, Option<mlua::Value>) = match expr {
            Expression::Parentheses { expression, .. } => self.type_of(expression)?,
            Expression::BinaryOperator { .. } | Expression::UnaryOperator { .. } => {
                self.vm.eval_expr_in_scope(&expr.to_string())?
            }
            Expression::FunctionCall(_) => self.vm.eval_expr_in_scope(&expr.to_string())?,
            // Primitives.
            Expression::Number(_)
            | Expression::String(_)
            | Expression::Symbol(_)
            | Expression::Var(_) => self.vm.eval_expr_in_scope(&expr.to_string())?,
            // Tables.
            Expression::TableConstructor(_) => self.vm.eval_expr_in_scope(&expr.to_string())?,
            // Function.
            Expression::Function(func) => {
                let body = &func.1;
                let params_count = body.parameters().iter().count();
                let mut results_count = 0;
                if let Some(LastStmt::Return(last_stmt)) = body.block().last_stmt() {
                    results_count = last_stmt.returns().iter().count();
                }

                let mut params = Vec::with_capacity(params_count);
                params.resize(params.capacity(), self.vm.any());
                let mut results = Vec::with_capacity(results_count);
                results.resize(results.capacity(), self.vm.any());

                (self.vm.function(&params, &results), None)
            }
            _ => unreachable!(),
        };

        Ok((t, literal))
    }

    fn assign(&mut self, lhs: Type, rhs: Type, range: (u32, u32)) {
        match self.vm.assign(lhs, rhs) {
            Ok(_) => {}
            Err(err) => self.errors.push(Error {
                message: err.to_string(),
                range,
            }),
        }
    }
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for Checker {
    fn visit_block(&mut self, _node: &full_moon::ast::Block) {
        self.vm.push_scope();
    }

    fn visit_block_end(&mut self, _node: &full_moon::ast::Block) {
        self.vm.pop_scope();
    }

    fn visit_do(&mut self, _node: &full_moon::ast::Do) {
        self.vm.push_scope();
    }

    fn visit_do_end(&mut self, _node: &full_moon::ast::Do) {
        self.vm.pop_scope();
    }

    fn visit_function_body(&mut self, _node: &full_moon::ast::FunctionBody) {
        self.vm.push_scope();
    }

    fn visit_function_body_end(&mut self, _node: &full_moon::ast::FunctionBody) {
        self.vm.pop_scope();
    }

    fn visit_if(&mut self, _node: &full_moon::ast::If) {
        self.vm.push_scope();
    }

    fn visit_if_end(&mut self, _node: &full_moon::ast::If) {
        self.vm.pop_scope();
    }

    fn visit_else_if(&mut self, _node: &full_moon::ast::ElseIf) {
        self.vm.push_scope();
    }

    fn visit_else_if_end(&mut self, _node: &full_moon::ast::ElseIf) {
        self.vm.pop_scope();
    }

    fn visit_local_assignment(&mut self, node: &full_moon::ast::LocalAssignment) {
        let (prec, _) = node.surrounding_trivia();

        let mut annotations = Vec::new();
        for p in prec {
            match p.token_type() {
                full_moon::tokenizer::TokenType::SingleLineComment { comment } => {
                    if let Some(expr) = comment.trim().strip_prefix("@") {
                        let t = match self.vm.eval_type(expr) {
                            Ok(t) => t,
                            Err(err) => {
                                self.errors.push(Error {
                                    message: err.to_string(),
                                    range: (
                                        p.start_position().bytes() as u32,
                                        p.end_position().bytes() as u32,
                                    ),
                                });
                                continue;
                            }
                        };
                        annotations.push(t);
                    }
                }
                _ => continue,
            }
        }

        let scope = self.vm.scope();
        for (i, (name, expr)) in std::iter::zip(node.names(), node.expressions()).enumerate() {
            let (t, value) = match self.type_of(expr) {
                Ok(r) => r,
                Err(err) => {
                    self.errors.push(Error {
                        message: err.to_string(),
                        range: (
                            name.start_position().unwrap().bytes() as u32,
                            expr.end_position().unwrap().bytes() as u32,
                        ),
                    });
                    return;
                }
            };

            if i < annotations.len() {
                self.assign(
                    annotations[i].clone(),
                    t.clone(),
                    (
                        name.start_position().unwrap().bytes() as u32,
                        expr.end_position().unwrap().bytes() as u32,
                    ),
                )
            }

            scope.set(&name.token().to_string(), t, value)
        }
    }

    fn visit_assignment(&mut self, node: &full_moon::ast::Assignment) {
        let scope = self.vm.scope();
        for (name, expr) in std::iter::zip(node.variables(), node.expressions()) {
            let lhs = match name {
                full_moon::ast::Var::Expression(expr) => {
                    match self.vm.eval_expr_in_scope(&expr.to_string()) {
                        Ok((t, _)) => t,
                        Err(err) => {
                            self.errors.push(Error {
                                message: err.to_string(),
                                range: (
                                    name.start_position().unwrap().bytes() as u32,
                                    expr.end_position().unwrap().bytes() as u32,
                                ),
                            });
                            return;
                        }
                    }
                }
                full_moon::ast::Var::Name(n) => scope.get_type(&n.token().to_string()),
                _ => unreachable!(),
            };

            let (rhs, _) = self.type_of(expr).unwrap();
            self.assign(
                lhs,
                rhs,
                (
                    name.start_position().unwrap().bytes() as u32,
                    expr.end_position().unwrap().bytes() as u32,
                ),
            );
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub range: (u32, u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_assign() {
        let mut checker = Checker::new();
        let ast = full_moon::parse(
            r#"
local foo, bar = 3.14, "baz"
foo = bar
bar = foo
"#,
        )
        .unwrap();

        let errs = checker.check(&ast).unwrap_err();

        assert_eq!(errs[0], "type string is not assignable to type number");
        assert_eq!(errs[1], "type number is not assignable to type string");
    }

    #[test]
    fn local_shadowing() {
        let mut checker = Checker::new();
        let ast = full_moon::parse(
            r#"
local foo, bar = 3.14, "baz"
do
    local foo = bar
end
"#,
        )
        .unwrap();

        checker.check(&ast).unwrap();
    }

    #[test]
    fn local_assign_expr() {
        let mut checker = Checker::new();
        let ast = full_moon::parse(
            r#"
local foo = 1 == 1
foo = true
foo = false
"#,
        )
        .unwrap();

        checker.check(&ast).unwrap();
    }

    #[test]
    fn local_annotation() {
        let mut checker = Checker::new();
        let ast = full_moon::parse(
            r#"
--@ fn(number, number) (number)
local foo = 3"#,
        )
        .unwrap();

        let errs = checker.check(&ast).unwrap_err();
        assert_eq!(
            errs[0],
            "type number is not assignable to type fn(number, number) (number)"
        );
    }

    #[test]
    fn annotation_after_decl() {
        let mut checker = Checker::new();
        let ast = full_moon::parse(
            r#"
local foo = 3
--@ string
foo = 'bar'"#,
        )
        .unwrap();

        let errs = checker.check(&ast).unwrap_err();
        assert_eq!(errs[0], "type string is not assignable to type number");
    }

    #[test]
    fn function_inference() {
        let mut checker = Checker::new();
        let ast = full_moon::parse(
            r#"
local foo = function(foo, bar)
    return foo
end
foo = "bar"
"#,
        )
        .unwrap();

        let errs = checker.check(&ast).unwrap_err();
        assert_eq!(
            errs[0],
            "type string is not assignable to type fn(any, any) (any)"
        );
    }
}
