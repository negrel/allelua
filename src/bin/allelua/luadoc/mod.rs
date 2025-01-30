use full_moon::visitors::Visitor;

mod lang;
mod luatype;
mod scope;

pub use lang::*;
pub use luatype::*;
pub use scope::*;

#[derive(Debug)]
pub struct LuaDoc {}

impl Visitor for LuaDoc {}
