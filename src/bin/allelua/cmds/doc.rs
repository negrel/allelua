use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use full_moon::{
    ast::{punctuated::Pair, LastStmt, Prefix, Var},
    tokenizer::{Token, TokenType},
    visitors::Visitor,
};
use walkdir::WalkDir;

use super::is_dir_or_lua_file;

pub fn doc(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    for path in paths {
        let iter = WalkDir::new(path)
            .into_iter()
            .filter_entry(is_dir_or_lua_file);

        for entry in iter {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }

            let fpath = entry.into_path();

            let source = fs::read_to_string(&fpath)
                .with_context(|| format!("failed to read lua file {fpath:?}"))?;
            let ast = full_moon::parse(&source)
                .with_context(|| format!("failed to parse lua file {fpath:?}"))?;

            let mut doc_visitor = LuaDocVisitor::new(&fpath, &source);
            doc_visitor.visit_ast(&ast);

            println!("{:?}", doc_visitor.module_doc());
        }
    }

    Ok(())
}

#[derive(Debug)]
struct LuaDocVisitor<'a> {
    source_path: &'a Path,
    source: &'a str,

    module_doc: Option<LuaModuleDoc>,

    // Current luadoc block we're building.
    // It is erased on every new token.
    luadoc_block: Option<String>,
}

impl<'a> LuaDocVisitor<'a> {
    pub fn new(source_path: &'a Path, source: &'a str) -> Self {
        Self {
            source,
            source_path,
            module_doc: None,
            luadoc_block: None,
        }
    }

    pub fn module_doc(self) -> Option<LuaModuleDoc> {
        self.module_doc
    }
}

impl<'a> Visitor for LuaDocVisitor<'a> {
    fn visit_token(&mut self, token: &Token) {
        // println!("{:?} -> {:?}", self.luadoc_block, token.token_type());

        match token.token_type() {
            TokenType::MultiLineComment { .. }
            | TokenType::SingleLineComment { .. }
            | TokenType::Whitespace { .. } => return,
            _ => {}
        }

        // Reset luadoc, this function is called before the specialized
        // visit_xxx function.
        if let Some(luadoc_block) = &self.luadoc_block {
            if luadoc_block.contains("@module") {
                let module_doc = luadoc_block
                    .lines()
                    .filter(|l| !l.contains("@module"))
                    .collect::<Vec<&str>>()
                    .join("");

                let module_identifier = luadoc_block
                    .lines()
                    .find(|l| l.contains("@module"))
                    .unwrap();
                let module_identifier = module_identifier
                    .split("@module ")
                    .nth(1)
                    .unwrap_or(self.source_path.to_str().unwrap());

                self.module_doc = Some(LuaModuleDoc {
                    identifier: module_identifier.to_owned(),
                    doc: module_doc,
                    functions: Vec::new(),
                });
            }
        }
        self.luadoc_block = None
    }

    fn visit_last_stmt(&mut self, node: &LastStmt) {
        if let LastStmt::Return(stmt) = node {
            // println!("{stmt:?}");
        }
    }

    fn visit_assignment(&mut self, node: &full_moon::ast::Assignment) {
        if node.variables().len() != 1 || node.expressions().len() != 1 {
            return;
        }

        let var = node.variables().first().unwrap();
        let rhs_expr = node.expressions().first().unwrap().value();

        if let Pair::End(Var::Expression(lhs_expr)) = var {
            if let Prefix::Name(token) = lhs_expr.prefix() {
                if let TokenType::Identifier { identifier } = token.token_type() {
                    // TODO: support other module identifier than M.
                    if identifier.as_str() != "M" {
                        return;
                    }
                }
            }
        }

        // This is an assignment to M.
        println!("{rhs_expr}");
    }

    fn visit_function_declaration(&mut self, node: &full_moon::ast::FunctionDeclaration) {
        println!("fn decl {node:?}");
    }

    fn visit_local_function(&mut self, node: &full_moon::ast::LocalFunction) {
        println!("fn decl {node:?}");
    }

    fn visit_function_name(&mut self, node: &full_moon::ast::FunctionName) {
        println!("fn decl {node:?}");
    }

    fn visit_single_line_comment(&mut self, token: &Token) {
        let start_pos = token.start_position();
        let end_pos = token.end_position();
        let comment = &self.source[start_pos.bytes()..end_pos.bytes()];

        if let Some(doc) = comment.strip_prefix("---") {
            let doc_block = self.luadoc_block.clone().unwrap_or("".to_owned());
            self.luadoc_block = Some(doc_block + doc + "\n");
        } else {
            self.luadoc_block = None
        }
    }
}

#[derive(Debug, Default)]
struct LuaModuleDoc {
    identifier: String,
    doc: String,
    functions: Vec<LuaFunctionDoc>,
}

#[derive(Debug)]
struct LuaFunctionDoc {
    doc: String,
    name: String,
    parameters: Vec<String>,
}
