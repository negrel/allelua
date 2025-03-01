use core::fmt;

use crate::Type;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TypeError {
    #[error("Operator '{op}' cannot be applied to types '{lhs}' and '{rhs}'")]
    BinOpNotSupported { lhs: Type, op: BinOp, rhs: Type },
}

/// BinOp define all Lua operators that require two operands.
#[derive(Debug, PartialEq, Eq)]
pub enum BinOp {
    Pow, // ^
    Div, // /
    Mod, // %
    Mul, // *
    Add, // +
    Sub, // -
    Cat, // ..
    Lt,  // <
    Le,  // <=
    Gt,  // >
    Ge,  // >=
    Ne,  // ~=
    Eq,  // ==
    And, // and
    Or,  // or
}

impl From<&full_moon::ast::BinOp> for BinOp {
    fn from(value: &full_moon::ast::BinOp) -> Self {
        match value {
            full_moon::ast::BinOp::Caret(_) => BinOp::Pow,
            full_moon::ast::BinOp::Percent(_) => BinOp::Mod,
            full_moon::ast::BinOp::Slash(_) => BinOp::Div,
            full_moon::ast::BinOp::Star(_) => BinOp::Mul,
            full_moon::ast::BinOp::Minus(_) => BinOp::Sub,
            full_moon::ast::BinOp::Plus(_) => BinOp::Add,
            full_moon::ast::BinOp::TwoDots(_) => BinOp::Cat,
            full_moon::ast::BinOp::GreaterThan(_) => BinOp::Gt,
            full_moon::ast::BinOp::GreaterThanEqual(_) => BinOp::Ge,
            full_moon::ast::BinOp::LessThan(_) => BinOp::Lt,
            full_moon::ast::BinOp::LessThanEqual(_) => BinOp::Le,
            full_moon::ast::BinOp::TildeEqual(_) => BinOp::Ne,
            full_moon::ast::BinOp::TwoEqual(_) => BinOp::Eq,
            full_moon::ast::BinOp::And(_) => BinOp::Add,
            full_moon::ast::BinOp::Or(_) => BinOp::Or,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            BinOp::Pow => "^",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Mul => "*",
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Cat => "..",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Ne => "~=",
            BinOp::Eq => "==",
            BinOp::And => "and",
            BinOp::Or => "or",
        };
        f.write_str(str)
    }
}
