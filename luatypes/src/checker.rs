use full_moon::{
    ast::{Ast, Expression},
    node::Node,
    tokenizer::Symbol,
    visitors::Visitor,
};
use holylib::types::{Type, VM};

/// Checker define a Lua type checker.
#[derive(Debug)]
pub struct Checker {
    vm: VM,
    errors: Vec<String>,
}

impl Checker {
    pub fn new() -> Self {
        Self {
            vm: VM::new().unwrap(),
            errors: Vec::new(),
        }
    }

    pub fn check(mut self, ast: &Ast) -> Result<(), Vec<String>> {
        self.visit_ast(ast);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    fn type_of(&self, expr: &Expression) -> mlua::Result<Type> {
        let t: Type = match expr {
            Expression::Parentheses { expression, .. } => self.type_of(expression)?,
            Expression::BinaryOperator { .. } | Expression::UnaryOperator { .. } => self
                .vm
                .eval_in_scope(&("return ".to_owned() + &expr.to_string()))?,
            Expression::FunctionCall(func_call) => match func_call.prefix() {
                full_moon::ast::Prefix::Expression(expr) => self.type_of(expr)?,
                full_moon::ast::Prefix::Name(_n) => {
                    todo!()
                }
                _ => unreachable!(),
            },
            // Primitives.
            Expression::Number(_) => self.vm.number(),
            Expression::String(_) => self.vm.string(),
            Expression::Symbol(tok) => match tok.token_type() {
                full_moon::tokenizer::TokenType::Symbol { symbol } => match symbol {
                    Symbol::Nil => self.vm.nil(),
                    Symbol::True | Symbol::False => self.vm.boolean(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            // Tables.
            Expression::TableConstructor(_tab) => {
                todo!()
            }
            // Function.
            Expression::Function(_func) => {
                todo!()
            }
            // Variable.
            Expression::Var(var) => match var {
                full_moon::ast::Var::Expression(e) => self.vm.eval_expr_in_scope(&e.to_string())?,
                full_moon::ast::Var::Name(n) => self.vm.scope().get(&n.token().to_string()),
                _ => unreachable!(),
            },

            _ => unreachable!(),
        };

        Ok(t)
    }

    fn assign(&mut self, lhs: Type, rhs: Type) {
        match self.vm.assign(lhs, rhs) {
            Ok(_) => {}
            Err(err) => self.errors.push(err.to_string()),
        }
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
                                self.errors.push(err.to_string());
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
            let t = self.type_of(expr).unwrap();

            if i < annotations.len() {
                self.assign(annotations[i].clone(), t.clone())
            }
            scope.set(&name.token().to_string(), t)
        }
    }

    fn visit_assignment(&mut self, node: &full_moon::ast::Assignment) {
        let scope = self.vm.scope();
        for (name, expr) in std::iter::zip(node.variables(), node.expressions()) {
            match name {
                full_moon::ast::Var::Expression(expr) => {
                    match self.vm.eval_expr_in_scope(&expr.to_string()) {
                        Ok(_) => todo!(),
                        Err(_) => todo!(),
                    }
                }
                full_moon::ast::Var::Name(n) => {
                    let lhs = scope.get(&n.token().to_string());
                    let rhs = self.type_of(expr).unwrap();
                    self.assign(lhs, rhs);
                }
                _ => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_assign() {
        let checker = Checker::new();

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
        let checker = Checker::new();

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
        let checker = Checker::new();

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
    fn local_assign_function() {
        let checker = Checker::new();

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
}
