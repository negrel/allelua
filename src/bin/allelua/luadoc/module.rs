use std::{fs, io, path::PathBuf};

use full_moon::ast::Ast;

use super::TypeChecker;

/// Module define a Lua module.
#[derive(Debug)]
pub struct Module {
    pub path: PathBuf,
    pub type_checker: TypeChecker,
}

impl Module {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            type_checker: TypeChecker::new(),
        }
    }

    pub fn parse(&self) -> Result<Ast, ModuleError> {
        Ok(full_moon::parse(&fs::read_to_string(&self.path)?)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{}", self)]
    Parse(Vec<full_moon::Error>),
}

impl From<Vec<full_moon::Error>> for ModuleError {
    fn from(value: Vec<full_moon::Error>) -> Self {
        Self::Parse(value)
    }
}
