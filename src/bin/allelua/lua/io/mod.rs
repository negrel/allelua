use std::slice;

use mlua::{AnyUserData, FromLua, IntoLua, Lua, ObjectLike};

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

pub const DEFAULT_BUFFER_SIZE: usize = 4 * 1024; // 4 KiB

pub fn load_io(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "io",
        lua.create_function(|lua, ()| {
            let io = lua.create_table()?;
            lua.globals().set("io", io.clone())?;

            io.set("default_buffer_size", DEFAULT_BUFFER_SIZE)?;

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

            let byte_search = lua.create_function(
                |_lua, (buf, str, start): (LuaJitBuffer, mlua::String, usize)| {
                    if str.as_bytes().len() != 1 {
                        return Err(mlua::Error::runtime("string must contain a single byte"));
                    }
                    let haystack = str.as_bytes()[0];

                    let bytes = &buf.ref_bytes()?[start..];
                    Ok(bytes.iter().enumerate().find_map(|(i, b)| {
                        if *b == haystack {
                            Some(start + i + 1)
                        } else {
                            None
                        }
                    }))
                },
            )?;

            lua.load(include_lua!("./io.lua"))
                .eval::<mlua::Function>()?
                .call::<()>((io.to_owned(), byte_search))?;

            Ok(io)
        })?,
    )
}

/// LuaJitBuffer is a wrapper around the LuaJIT string.buffer extension stored
/// as a [mlua::AnyUserData].
#[derive(Debug, Clone)]
pub struct LuaJitBuffer {
    udata: mlua::AnyUserData,
}

impl FromLua for LuaJitBuffer {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        let type_name = value.type_name();
        let udata = AnyUserData::from_lua(value, lua)?;
        // If it is a LuaJIT buffer userdata, we shouldn't be able to
        // retrieve it's metatable via mlua.
        match udata.metatable() {
            Err(mlua::Error::UserDataTypeMismatch) => Ok(Self { udata }),
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
        Self::new_with_capacity(lua, DEFAULT_BUFFER_SIZE)
    }

    #[allow(dead_code)]
    pub fn new_with_capacity(lua: Lua, n: usize) -> mlua::Result<Self> {
        let udata = lua
            .globals()
            .get::<mlua::Table>("string")?
            .get::<mlua::Table>("buffer")?
            .call_function::<AnyUserData>("new", n)?;

        Ok(Self { udata })
    }

    pub fn ref_bytes(&self) -> mlua::Result<&[u8]> {
        let (ptr, len) = self.udata.call_method::<(mlua::Value, usize)>("ref", ())?;

        if len == 0 || ptr.is_null() {
            Ok(&[])
        } else {
            let ptr = unsafe { *(ptr.to_pointer() as *const *const u8) };
            if ptr.is_null() {
                Ok(&[])
            } else {
                unsafe { Ok(slice::from_raw_parts(ptr, len)) }
            }
        }
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
