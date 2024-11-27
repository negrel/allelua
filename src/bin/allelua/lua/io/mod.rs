use std::slice;

use mlua::{chunk, AnyUserData, FromLua, IntoLua, Lua, ObjectLike, UserData};

use crate::include_lua;

mod bufio;
mod closer;
mod error;
mod maybe_buffered;
mod reader;
mod seeker;
mod writer;

pub use bufio::*;
pub use closer::*;
pub use error::*;
pub use maybe_buffered::*;
pub use reader::*;
pub use seeker::*;
pub use writer::*;

pub const DEFAULT_BUF_SIZE: usize = 8 * 1024;

pub fn load_io(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "io",
        lua.create_function(|lua, ()| {
            let io = lua.create_table()?;
            lua.globals().set("io", io.clone())?;

            let errors = lua.create_table()?;
            errors.set(
                "closed",
                super::error::LuaError::from(LuaIoClosedError).into_lua(lua)?,
            )?;
            io.set("errors", errors)?;

            let seek_from_constructors = lua.create_table()?;
            seek_from_constructors.set(
                "start",
                lua.create_function(|_, n: u64| Ok(LuaSeekFrom::start(n)))?,
            )?;
            seek_from_constructors.set(
                "current",
                lua.create_function(|_, n: i64| Ok(LuaSeekFrom::current(n)))?,
            )?;
            seek_from_constructors.set(
                "end",
                lua.create_function(|_, n: i64| Ok(LuaSeekFrom::end(n)))?,
            )?;
            io.set("SeekFrom", seek_from_constructors)?;

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
pub struct LuaJitBuffer {
    udata: mlua::AnyUserData,
    #[allow(dead_code)]
    lua: Lua,
}

impl FromLua for LuaJitBuffer {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        let type_name = value.type_name();
        let udata = AnyUserData::from_lua(value, lua)?;
        // If it is a LuaJIT buffer userdata, we shouldn't be able to
        // retrieve it's metatable via mlua.
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
    pub fn new(lua: Lua) -> mlua::Result<Self> {
        Self::new_with_capacity(lua, DEFAULT_BUF_SIZE)
    }

    pub fn new_with_capacity(lua: Lua, n: usize) -> mlua::Result<Self> {
        let udata = lua
            .load(chunk! {
                return require("string.buffer").new($n)
            })
            .eval::<mlua::AnyUserData>()?;

        Ok(Self { udata, lua })
    }

    pub fn as_bytes(&self) -> mlua::Result<&mut [u8]> {
        let (ptr, len) = self.udata.call_method::<(mlua::Value, usize)>("ref", ())?;

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

    pub fn as_boxed_slice(&self) -> mlua::Result<Box<[u8]>> {
        let bytes = self.as_bytes()?;
        unsafe { Ok(Box::from_raw(bytes as *mut [u8])) }
    }

    pub fn reserve_bytes(&self, n: usize) -> mlua::Result<&mut [u8]> {
        let (ptr, len) = self
            .udata
            .call_method::<(mlua::Value, usize)>("reserve", n)?;

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

    pub fn skip(&self, n: usize) -> mlua::Result<()> {
        if n == 0 {
            return Ok(());
        }

        self.udata.call_method("skip", n)
    }

    pub fn commit(&self, n: usize) -> mlua::Result<()> {
        if n == 0 {
            return Ok(());
        }

        self.udata.call_method("commit", n)
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
