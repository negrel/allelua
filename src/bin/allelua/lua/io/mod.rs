use std::slice;

use mlua::{AnyUserData, FromLua, Lua, ObjectLike, UserData};

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

/// LuaJitBuffer is a wrapper around the LuaJIT string.buffer extension stored
/// as a [mlua::AnyUserData].
#[derive(Debug, Clone)]
struct LuaJitBuffer {
    udata: mlua::AnyUserData,
    #[allow(dead_code)]
    lua: Lua,
}

impl FromLua for LuaJitBuffer {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        let type_name = value.type_name();
        let udata = AnyUserData::from_lua(value, lua)?;
        // If it is a real LuaJIT buffer userdata, we shouldn't be able to
        // retrieve it's metatable.
        match udata.metatable() {
            Err(mlua::Error::UserDataTypeMismatch) => Ok(Self {
                udata,
                lua: lua.to_owned(),
            }),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: type_name,
                to: "buffer".to_string(),
                message: None,
            }),
        }
    }
}

impl LuaJitBuffer {
    fn as_bytes(&self) -> mlua::Result<&mut [u8]> {
        let (ptr, len) = self
            .udata
            .get::<mlua::Function>("ref")?
            .call::<(mlua::Value, usize)>(mlua::Value::UserData(self.udata.to_owned()))?;

        if len == 0 || ptr.is_null() {
            Ok(&mut [])
        } else {
            let ptr = unsafe { *(ptr.to_pointer() as *const *mut u8) };
            if ptr.is_null() {
                Ok(&mut [])
            } else {
                unsafe { Ok(slice::from_raw_parts_mut(ptr, len)) }
            }
        }
    }

    fn reserve_bytes(&self, n: usize) -> mlua::Result<&mut [u8]> {
        let (ptr, len) = self
            .udata
            .get::<mlua::Function>("reserve")?
            .call::<(mlua::Value, usize)>((mlua::Value::UserData(self.udata.to_owned()), n))?;

        if len == 0 || ptr.is_null() {
            Ok(&mut [])
        } else {
            let ptr = unsafe { *(ptr.to_pointer() as *const *mut u8) };
            if ptr.is_null() {
                Ok(&mut [])
            } else {
                unsafe { Ok(slice::from_raw_parts_mut(ptr, len)) }
            }
        }
    }

    fn skip(&self, n: usize) -> mlua::Result<()> {
        if n == 0 {
            return Ok(());
        }

        self.udata
            .get::<mlua::Function>("skip")?
            .call::<()>((self.udata.to_owned(), n))
    }

    fn commit(&self, n: usize) -> mlua::Result<()> {
        if n == 0 {
            return Ok(());
        }

        self.udata
            .get::<mlua::Function>("commit")?
            .call::<()>((self.udata.to_owned(), n))
    }
}

/// LuaBuffer is a wrapper around a byte slice that is not converted to a table
/// when passed to Lua function. This is used to minimize copy when passing data
/// from a reader to a writer.
#[derive(Debug)]
struct LuaBuffer<'a>(&'a [u8]);

impl LuaBuffer<'static> {
    pub unsafe fn new_static(buf: &[u8]) -> Self {
        let buf: &'static [u8] = std::mem::transmute(buf);
        Self(buf)
    }

    pub fn as_bytes(&self) -> &'static [u8] {
        self.0
    }
}

impl UserData for LuaBuffer<'static> {}

impl FromLua for LuaBuffer<'static> {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        let udata = AnyUserData::from_lua(value, lua)?;
        udata.take()
    }
}
