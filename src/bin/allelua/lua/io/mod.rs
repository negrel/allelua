use mlua::{AnyUserData, FromLua, Lua, UserData};

use crate::include_lua;

mod closer;
mod error;
mod reader;
mod seeker;
mod writer;

pub use closer::*;
pub use error::*;
pub use reader::*;
pub use seeker::*;
pub use writer::*;

pub fn load_io(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "io",
        lua.create_function(|lua, ()| {
            let io = lua.create_table()?;

            lua.load(include_lua!("./io.lua"))
                .eval::<mlua::Function>()?
                .call::<()>(io.to_owned())?;

            Ok(io)
        })?,
    )
}

#[derive(Debug)]
pub struct LuaBuffer<'a>(&'a [u8]);

impl LuaBuffer<'static> {
    pub unsafe fn new_static(buf: &[u8]) -> Self {
        let buf: &'static [u8] = std::mem::transmute(buf);
        Self(buf)
    }
}

impl UserData for LuaBuffer<'static> {}

impl FromLua for LuaBuffer<'static> {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        let udata = AnyUserData::from_lua(value, lua)?;

        udata.take()
    }
}
