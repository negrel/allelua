use std::{ffi::OsString, fmt::Display, ops::Deref, path::Path, process::exit};

use error::load_error;
use mlua::{chunk, AsChunk, FromLuaMulti, Lua, LuaOptions, StdLib};

use self::{
    coroutine::load_coroutine, globals::register_globals, io::load_io, os::load_os,
    package::load_package, path::load_path, sh::load_sh, string::load_string, sync::load_sync,
    table::load_table, test::load_test, time::load_time,
};

mod coroutine;
mod error;
mod globals;
mod io;
mod os;
mod package;
mod path;
mod sh;
mod string;
mod sync;
mod table;
mod test;
mod time;

#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeSafetyLevel {
    Unsafe,
    Safe,
}

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
    pub fn new(fpath: &Path, run_args: Vec<OsString>, safety: RuntimeSafetyLevel) -> Self {
        let mut stdlib = StdLib::NONE
            | StdLib::MATH
            | StdLib::TABLE
            | StdLib::PACKAGE
            | StdLib::BIT
            | StdLib::STRING
            | StdLib::DEBUG;

        if safety == RuntimeSafetyLevel::Unsafe {
            stdlib = stdlib | StdLib::FFI | StdLib::JIT;
        }

        let vm = unsafe { Lua::unsafe_new_with(stdlib, LuaOptions::new()) };

        if fpath.is_relative() {
            panic!("convert path to absolute before creating runtime");
        }

        prepare_runtime(vm.clone(), fpath, run_args, safety);

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

fn prepare_runtime(lua: Lua, fpath: &Path, run_args: Vec<OsString>, safety: RuntimeSafetyLevel) {
    // Load libraries.
    handle_result(load_path(lua.clone()));
    handle_result(load_os(&lua, run_args));
    handle_result(load_error(lua.clone()));
    handle_result(load_sync(lua.clone()));
    handle_result(load_io(&lua));
    handle_result(load_string(lua.clone()));
    handle_result(load_coroutine(lua.clone()));
    handle_result(load_table(lua.clone()));
    handle_result(load_sh(&lua));
    handle_result(load_time(lua.clone()));
    handle_result(register_globals(lua.clone()));

    if safety == RuntimeSafetyLevel::Unsafe {
        handle_result(load_test(lua.clone()));
    }

    // overwrite require.
    handle_result(load_package(lua.clone(), fpath));

    let result = lua
        .load(chunk! {
            local package = require("package")
            local table = require("table")

            // Hide debug module.
            _G.debug = nil
            package.loaded.debug = nil

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
