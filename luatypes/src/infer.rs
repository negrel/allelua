use full_moon::{ast::Expression, tokenizer::Symbol};

use crate::{
    error::TypeError, registry::TypeRegistry, InterfaceType, LiteralType, PrimitiveType, Type,
};

/// InferEngine define a type inference engine. Identifier type resolution is
/// delegated to [TypeRegistry], this type doesn't handle scopes directly.
#[derive(Debug, Default)]
pub struct InferEngine {
    registry: TypeRegistry,
}

impl InferEngine {
    pub fn type_of(&self, expr: &Expression) -> Result<Type, TypeError> {
        let t: Type = match expr {
            Expression::BinaryOperator { lhs, binop, rhs } => {
                let lhs = self.type_of(lhs)?;
                let rhs = self.type_of(rhs)?;

                let res = match binop {
                    full_moon::ast::BinOp::Caret(_) => lhs.pow(&rhs),
                    full_moon::ast::BinOp::Percent(_) => lhs.modulo(&rhs),
                    full_moon::ast::BinOp::Slash(_) => lhs.div(&rhs),
                    full_moon::ast::BinOp::Star(_) => lhs.mul(&rhs),
                    full_moon::ast::BinOp::Minus(_) => lhs.sub(&rhs),
                    full_moon::ast::BinOp::Plus(_) => lhs.add(&rhs),
                    full_moon::ast::BinOp::TwoDots(_) => lhs.concat(&rhs),
                    full_moon::ast::BinOp::GreaterThan(_) => todo!(),
                    full_moon::ast::BinOp::GreaterThanEqual(_) => todo!(),
                    full_moon::ast::BinOp::LessThan(_) => todo!(),
                    full_moon::ast::BinOp::LessThanEqual(_) => todo!(),
                    full_moon::ast::BinOp::TildeEqual(_) => todo!(),
                    full_moon::ast::BinOp::TwoEqual(_) => todo!(),
                    full_moon::ast::BinOp::And(_) => todo!(),
                    full_moon::ast::BinOp::Or(_) => todo!(),
                    _ => unreachable!(),
                };

                match res {
                    Some(t) => t,
                    None => {
                        return Err(TypeError::BinOpNotSupported {
                            lhs,
                            op: binop.into(),
                            rhs,
                        })
                    }
                }
            }
            Expression::Parentheses {
                contained,
                expression,
            } => todo!(),
            Expression::UnaryOperator { unop, expression } => todo!(),
            Expression::Function(func) => {
                dbg!(func);
                todo!()
            }
            Expression::FunctionCall(func_call) => match func_call.prefix() {
                full_moon::ast::Prefix::Expression(expr) => self.type_of(expr)?,
                full_moon::ast::Prefix::Name(n) => {
                    dbg!(n);
                    todo!()
                }
                _ => unreachable!(),
            },
            Expression::TableConstructor(tab) => {
                let mut fields = Vec::new();
                for f in tab.fields() {
                    match f {
                        full_moon::ast::Field::ExpressionKey {
                            brackets: _,
                            key,
                            equal: _,
                            value,
                        } => {
                            let key = self.type_of(key)?;
                            let value = self.type_of(value)?;
                            if value == Type::Primitive(PrimitiveType::Nil) {
                                continue;
                            }
                            fields.push((key, value))
                        }
                        full_moon::ast::Field::NameKey {
                            key,
                            equal: _,
                            value,
                        } => match key.token_type() {
                            full_moon::tokenizer::TokenType::Identifier { identifier } => {
                                let value = self.type_of(value)?;
                                if value == Type::Primitive(PrimitiveType::Nil) {
                                    continue;
                                }

                                fields.push((
                                    Type::Literal(LiteralType::string(identifier.as_str())),
                                    value,
                                ))
                            }
                            _ => unreachable!(),
                        },
                        full_moon::ast::Field::NoKey(_) => todo!(),
                        _ => unreachable!(),
                    }
                }

                Type::Interface(InterfaceType::from_iter(fields))
            }
            Expression::Number(tok) => Type::Literal(LiteralType::number(match tok.token_type() {
                full_moon::tokenizer::TokenType::Number { text } => text.as_str(),
                _ => unreachable!(),
            })),
            Expression::String(tok) => Type::Literal(LiteralType::string(match tok.token_type() {
                full_moon::tokenizer::TokenType::StringLiteral { literal, .. } => literal.as_str(),
                _ => unreachable!(),
            })),
            Expression::Symbol(tok) => match tok.token_type() {
                full_moon::tokenizer::TokenType::Symbol { symbol } => match symbol {
                    Symbol::Nil => Type::Primitive(PrimitiveType::Nil),
                    Symbol::True => Type::Literal(LiteralType::boolean("true")),
                    Symbol::False => Type::Literal(LiteralType::boolean("false")),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            Expression::Var(var) => match var {
                full_moon::ast::Var::Expression(_) => todo!(),
                full_moon::ast::Var::Name(n) => {
                    dbg!(n);
                    todo!();
                }
                // match n.token_type() {
                //     full_moon::tokenizer::TokenType::Identifier { identifier } => self
                //         .scopes
                //         .get_var(identifier.as_str())
                //         .map(|v| v.lua_type.clone())
                //         .unwrap_or(Type::from(PrimitiveType::Nil)),
                //     _ => unreachable!(),
                // },
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use full_moon::ast::Expression;

    use crate::error::BinOp;

    use super::*;

    fn parse_expr(str: &str) -> Expression {
        let ast = full_moon::parse(str).unwrap();
        match ast.nodes().last_stmt().unwrap() {
            full_moon::ast::LastStmt::Return(ret) => {
                ret.returns().first().unwrap().value().to_owned()
            }
            _ => unreachable!(),
        }
    }

    #[test]
    pub fn type_of_literal_number() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::number("3.14")),
            infer.type_of(&parse_expr("return 3.14")).unwrap()
        );
    }

    #[test]
    pub fn type_of_literal_string() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::string("3.14")),
            infer.type_of(&parse_expr(r#"return "3.14""#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_nil() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Primitive(PrimitiveType::Nil),
            infer.type_of(&parse_expr(r#"return nil"#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_literal_true() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::boolean("true")),
            infer.type_of(&parse_expr(r#"return true"#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_literal_false() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::boolean("false")),
            infer.type_of(&parse_expr(r#"return false"#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_addition_of_literal_numbers() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::number("3")),
            infer.type_of(&parse_expr(r#"return 1 + 2"#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_addition_of_literal_number_and_string() {
        let infer = InferEngine::default();
        assert_eq!(
            TypeError::BinOpNotSupported {
                lhs: Type::Literal(LiteralType::number("1")),
                op: BinOp::Add,
                rhs: Type::Literal(LiteralType::string("2"))
            },
            infer.type_of(&parse_expr(r#"return 1 + "2""#)).unwrap_err()
        );
    }

    #[test]
    pub fn type_of_substraction_of_literal_numbers() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::number("-1")),
            infer.type_of(&parse_expr(r#"return 1 - 2"#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_multiplication_of_literal_numbers() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::number("2")),
            infer.type_of(&parse_expr(r#"return 1 * 2"#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_division_of_literal_numbers() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::number("0.5")),
            infer.type_of(&parse_expr(r#"return 1 / 2"#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_modulo_of_literal_numbers() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::number("1.1400001")),
            infer.type_of(&parse_expr(r#"return 3.14 % 2"#)).unwrap()
        );
    }

    #[test]
    pub fn type_of_concat_of_literal_strings() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Literal(LiteralType::string("foobar")),
            infer
                .type_of(&parse_expr(r#"return "foo" .. 'bar'"#))
                .unwrap()
        );
    }

    #[test]
    pub fn type_of_table() {
        let infer = InferEngine::default();
        assert_eq!(
            Type::Interface(InterfaceType::from([
                (
                    Type::Literal(LiteralType::string("str")),
                    Type::Literal(LiteralType::string("foo")),
                ),
                (
                    Type::Literal(LiteralType::string("num")),
                    Type::Literal(LiteralType::number("3.14")),
                ),
                (
                    Type::Literal(LiteralType::string("bool")),
                    Type::Literal(LiteralType::boolean("true")),
                ),
            ])),
            infer
                .type_of(&parse_expr(
                    r#"return { str = "foo", num = 3.14, bool = true, null = nil }"#
                ))
                .unwrap()
        );
    }
}
