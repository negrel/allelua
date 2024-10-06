use full_moon::{
    ast::{self, Expression},
    node::Node,
    visitors::Visitor,
};

pub struct LuaType(mlua::Table);

impl LuaType {
    // fn new() -> Self {}
}

pub fn typeof_module(ast: &ast::Ast) -> Option<LuaType> {
    let mut visitor = ModuleVisitor {};
    let last_stmt = ast.nodes().last_stmt()?;
    match last_stmt {
        ast::LastStmt::Break(_) => None,
        ast::LastStmt::Return(ret) => ret.returns(),
        _ => unreachable!(),
    }
}

pub fn typeof_punctuated_expr(punc: &ast::punctuated::Punctuated<Expression>) -> Option<LuaType> {
    for expr in punc.iter() {}
}

pub fn typeof_expr(expr: &Expression) -> Option<LuaType> {
    match expr {
        Expression::BinaryOperator { lhs, binop, rhs } => todo!(),
        Expression::Parentheses {
            contained,
            expression,
        } => todo!(),
        Expression::UnaryOperator { unop, expression } => todo!(),
        Expression::Function(_) => todo!(),
        Expression::FunctionCall(_) => todo!(),
        Expression::TableConstructor(_) => todo!(),
        Expression::Number(_) => todo!(),
        Expression::String(_) => todo!(),
        Expression::Symbol(_) => todo!(),
        Expression::Var(_) => todo!(),
        _ => todo!(),
    }
}

struct ModuleVisitor {}

impl Visitor for ModuleVisitor {
    fn visit_block(&mut self, node: &ast::Block) {
        println!("{node} {:?}", node.surrounding_trivia());
    }
}

#[cfg(test)]
mod tests {
    use crate::typeof_module;

    #[test]
    fn typeof_module_is_return_type() {
        let ast = full_moon::parse(
            r#"
                function greet(name)
                    return "Hello, " .. name .. "!"
                end

                do
                    print(greet("world!"))
                end

                return 3.14
            "#,
        )
        .unwrap();

        typeof_module(&ast);
    }
}
