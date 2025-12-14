use std::{ffi::c_void, iter::Iterator, pin::Pin};

use mlua::{AsChunk, LightUserData, Lua, LuaOptions, StdLib, Table};

use crate::Error;

/// Runtime represents an instance of the Allelua runtime including a Lua VM and
/// other associated resources.
#[derive(Debug)]
pub struct Runtime {
    inner: Pin<Box<Inner>>,
}

impl Runtime {
    /// Creates and initializes a new runtime.
    pub fn new(args: impl Iterator<Item = String>) -> Result<Self, Error> {
        let mut inner = Box::pin(Inner::new(args)?);
        let ptr = unsafe { Pin::get_unchecked_mut(inner.as_mut()) as *mut Inner as *mut c_void };
        inner.lua.set_named_registry_value(
            "__allelua",
            mlua::Value::LightUserData(LightUserData(ptr)),
        )?;

        Ok(Self { inner })
    }

    /// Loads and executes a chunk of Lua code.
    pub fn do_chunk(&mut self, chunk: impl AsChunk) -> Result<(), Error> {
        self.inner.do_chunk(chunk)
    }
}

/// Retrieves runtime associated to Lua VM. This panics if the Lua VM isn't
/// owned by the runtime.
impl From<Lua> for Runtime {
    fn from(lua: Lua) -> Self {
        let ludata = lua
            .named_registry_value::<mlua::LightUserData>("__allelua")
            .expect("Runtime::from(lua) using a different lua VM is not allowed");
        let inner: Pin<Box<Inner>> =
            unsafe { Pin::new_unchecked(Box::from_raw(ludata.0 as *mut Inner)) };

        Self { inner }
    }
}

/// Non pinned runtime.
#[derive(Debug)]
struct Inner {
    lua: Lua,
    rs: Table,
}

impl Inner {
    /// Creates and initializes a new runtime. Arguments will be passed to Lua
    /// main function.
    fn new(args: impl Iterator<Item = String>) -> Result<Self, Error> {
        let mut options = LuaOptions::default();
        options.catch_rust_panics = false;
        let lua = unsafe { Lua::unsafe_new_with(StdLib::ALL, options) };

        // Global table shared between Lua and Rust.
        let rs = lua.create_table()?;
        rs.set("args", lua.create_sequence_from(args.into_iter())?)?;

        lua.globals().set("_rs", rs.clone())?;

        lua.load(include_str!("./embed/00_string.lua"))
            .call::<()>(())?;
        lua.load(include_str!("./embed/99_start.lua"))
            .call::<()>(())?;

        Ok(Self { lua, rs })
    }

    /// Executes a chunk of Lua code within the runtime.
    fn do_chunk(&mut self, chunk: impl AsChunk) -> Result<(), Error> {
        let start = self.rs.get::<mlua::Function>("start")?;
        let chunk = self.lua.load(chunk);
        start.call::<()>(chunk.into_function()?)?;
        Ok(())
    }
}
