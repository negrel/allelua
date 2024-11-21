use std::path::PathBuf;

use full_moon::{ast, node::Node, visitors::Visitor};

mod luatype;
mod module;
mod scope;

use luatype::*;
use module::*;
use scope::*;

/// AstTypeEvaluator is a Lua AST visitor that partially evaluates code for type
/// inference and type checking.
#[derive(Debug)]
pub struct AstTypeEvaluator {
    module: Module,
    scopes: ScopeManager,
}

impl AstTypeEvaluator {
    pub fn new(path: PathBuf) -> Self {
        Self {
            module: Module::new(path),
            scopes: ScopeManager::default(),
        }
    }

    pub fn eval(&mut self) {
        self.visit_ast(&self.module.parse().unwrap())
    }
}

impl Visitor for AstTypeEvaluator {
    fn visit_local_function(&mut self, _node: &ast::LocalFunction) {}

    fn visit_function_declaration(&mut self, node: &ast::FunctionDeclaration) {
        let (leading_trivia, _) = node.surrounding_trivia();
        println!(
            "!! {}",
            leading_trivia
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    fn visit_function_body(&mut self, node: &ast::FunctionBody) {
        self.scopes.push_scope();

        for param in node.parameters() {
            match param {
                ast::Parameter::Ellipsis(tokref) | ast::Parameter::Name(tokref) => {
                    self.scopes.set_local(Variable {
                        definition: tokref.token().start_position()..tokref.token().end_position(),
                        identifier: tokref.to_string(),
                        type_id: TypeId::UNKNOWN,
                    });
                }
                _ => unreachable!(),
            }
        }
    }

    fn visit_function_body_end(&mut self, _node: &ast::FunctionBody) {
        self.scopes.pop_scope();
    }

    fn visit_block(&mut self, _node: &ast::Block) {
        self.scopes.push_scope();
    }
    fn visit_block_end(&mut self, _node: &ast::Block) {
        self.scopes.pop_scope();
    }
}
