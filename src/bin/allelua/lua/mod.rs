use std::{ffi::OsString, fmt::Display, ops::Deref, path::Path, process::exit};

use error::load_error;
use math::load_math;
use mlua::{chunk, AsChunk, FromLua, FromLuaMulti, IntoLua, Lua, LuaOptions, ObjectLike, StdLib};

use self::{
    coroutine::load_coroutine, globals::register_globals, io::load_io, json::load_json,
    os::load_os, package::load_package, path::load_path, perf::load_perf, sh::load_sh,
    string::load_string, sync::load_sync, table::load_table, test::load_test, time::load_time,
};

mod coroutine;
mod error;
mod globals;
mod io;
mod json;
mod math;
mod os;
mod package;
mod path;
mod perf;
mod sh;
mod string;
mod sync;
mod table;
mod test;
mod time;

/// Runtime define ready to use Lua VM with the allelua std lib loaded.
pub struct Runtime(Lua);

impl Drop for Runtime {
    fn drop(&mut self) {
        // Collect twice to ensure we collect all reachable objects.
        // This is necessary to finalize values owned by the runtime (e.g. close
        // LuaFile).
        self.0.gc_collect().unwrap();
        self.0.gc_collect().unwrap();
    }
}

impl Deref for Runtime {
    type Target = Lua;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Runtime {
    pub fn new(fpath: &Path, run_args: Vec<OsString>) -> Self {
        let stdlib = StdLib::NONE
            | StdLib::MATH
            | StdLib::TABLE
            | StdLib::PACKAGE
            | StdLib::BIT
            | StdLib::STRING
            | StdLib::JIT
            // Unsafe
            | StdLib::DEBUG
            | StdLib::FFI;

        let vm = unsafe { Lua::unsafe_new_with(stdlib, LuaOptions::new()) };

        if fpath.is_relative() {
            panic!("convert path to absolute before creating runtime");
        }

        prepare_runtime(vm.clone(), fpath, run_args);

        Runtime(vm)
    }

    pub async fn exec<'a, T>(&self, chunk: impl AsChunk<'_>) -> mlua::Result<T>
    where
        T: FromLuaMulti + 'a,
    {
        self.load(chunk).eval_async::<T>().await
    }
}

fn handle_result<T, E: Display>(result: Result<T, E>) {
    match result {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{err}");
            exit(-1);
        }
    }
}

fn prepare_runtime(lua: Lua, fpath: &Path, run_args: Vec<OsString>) {
    // Load libraries.
    handle_result(load_math(&lua));
    handle_result(load_path(&lua));
    handle_result(load_os(&lua, run_args));
    handle_result(load_error(&lua));
    handle_result(load_sync(&lua));
    handle_result(load_io(&lua));
    handle_result(load_string(&lua));
    handle_result(load_coroutine(&lua));
    handle_result(load_table(&lua));
    handle_result(load_sh(&lua));
    handle_result(load_time(&lua));
    handle_result(load_json(&lua));
    handle_result(load_perf(&lua));
    handle_result(register_globals(&lua));
    handle_result(load_test(lua.clone()));

    // overwrite require.
    handle_result(load_package(lua.clone(), fpath));

    let result = lua
        .load(chunk! {
            local package = require("package")
            local table = require("table")

            // Hide unsafe modules (debug, ffi).
            _G.debug = nil
            package.loaded.debug = nil
            _G.ffi = nil
            package.loaded.ffi = nil
            // Adds string.buffer as string/buffer for consistency
            package.loaded["string/buffer"] = package.loaded["string.buffer"]

            // Freeze modules.
            table.map(package.loaded, function(_k, v)
                return freeze(v)
            end)
        })
        .eval::<()>();

    handle_result(result);
}

/// IncludeChunk is an helper type used by include_lua macro.
pub struct IncludeChunk {
    pub name: String,
    pub source: &'static [u8],
}

impl<'a> mlua::AsChunk<'a> for IncludeChunk {
    fn source(self) -> std::io::Result<std::borrow::Cow<'a, [u8]>> {
        Ok(std::borrow::Cow::Borrowed(self.source))
    }

    fn name(&self) -> Option<String> {
        Some(self.name.clone())
    }

    fn environment(&self, _lua: &Lua) -> mlua::prelude::LuaResult<Option<mlua::prelude::LuaTable>> {
        Ok(None)
    }

    fn mode(&self) -> Option<mlua::ChunkMode> {
        Some(mlua::ChunkMode::Text)
    }
}

/// LuaInterface is a trait implemented by [mlua::UserData] types that implements
/// different Lua interface depending on their type parameters (e.g. [LuaFile<File>] and
/// [LuaFile<BufStream<File>>]).
pub trait LuaInterface: Sized {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M);
}

#[macro_export]
macro_rules! include_lua {
    ($path:tt) => {{
        let path = ::std::path::Path::new($path);
        $crate::lua::IncludeChunk {
            name: path.file_stem().unwrap().to_string_lossy().to_string(),
            source: include_bytes!($path),
        }
    }};
}

/// LuaObject is either a [mlua::Table] or [mlua::AnyUserData]. This wrapper
/// types is convenient when you want to manipulate an object and you don't care
/// about how it is implemented.
#[derive(Debug, Clone)]
pub enum LuaObject {
    Table(mlua::Table),
    UserData(mlua::AnyUserData),
}

impl FromLua for LuaObject {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        let obj = mlua::Either::<mlua::Table, mlua::AnyUserData>::from_lua(value, lua)?;
        match obj {
            mlua::Either::Left(table) => Ok(Self::Table(table)),
            mlua::Either::Right(udata) => Ok(Self::UserData(udata)),
        }
    }
}

impl IntoLua for LuaObject {
    fn into_lua(self, lua: &Lua) -> mlua::Result<mlua::Value> {
        match self {
            LuaObject::Table(tab) => tab.into_lua(lua),
            LuaObject::UserData(udata) => udata.into_lua(lua),
        }
    }
}

macro_rules! lua_obj_do {
    ($self:ident, $obj:ident => $do:expr) => {
        match $self {
            Self::Table($obj) => $do,
            Self::UserData($obj) => $do,
        }
    };
}

#[allow(dead_code)]
impl LuaObject {
    pub fn get<V: FromLua>(&self, key: impl IntoLua) -> mlua::Result<V> {
        lua_obj_do!(self, obj => { obj.get(key) })
    }

    pub fn set(&self, key: impl IntoLua, value: impl IntoLua) -> mlua::Result<()> {
        lua_obj_do!(self, obj => { obj.set(key, value) })
    }

    pub fn call<R>(&self, args: impl mlua::prelude::IntoLuaMulti) -> mlua::Result<R>
    where
        R: FromLuaMulti,
    {
        lua_obj_do!(self, obj => { obj.call(args) })
    }

    pub async fn call_async<R>(&self, args: impl mlua::IntoLuaMulti) -> mlua::Result<R>
    where
        R: FromLuaMulti,
    {
        lua_obj_do!(self, obj => { obj.call_async(args).await })
    }

    pub fn call_method<R>(&self, name: &str, args: impl mlua::IntoLuaMulti) -> mlua::Result<R>
    where
        R: FromLuaMulti,
    {
        lua_obj_do!(self, obj => { obj.call_method(name, args) })
    }

    pub async fn call_async_method<R>(
        &self,
        name: &str,
        args: impl mlua::IntoLuaMulti,
    ) -> mlua::Result<R>
    where
        R: FromLuaMulti,
    {
        lua_obj_do!(self, obj => { obj.call_async_method(name, args).await })
    }

    pub fn call_function<R>(&self, name: &str, args: impl mlua::IntoLuaMulti) -> mlua::Result<R>
    where
        R: FromLuaMulti,
    {
        lua_obj_do!(self, obj => { obj.call_function(name, args) })
    }

    pub async fn call_async_function<R>(
        &self,
        name: &str,
        args: impl mlua::IntoLuaMulti,
    ) -> mlua::Result<R>
    where
        R: FromLuaMulti,
    {
        lua_obj_do!(self, obj => { obj.call_async_function(name, args).await })
    }

    pub fn to_string(&self) -> mlua::Result<String> {
        lua_obj_do!(self, obj => { obj.to_string() })
    }
}
