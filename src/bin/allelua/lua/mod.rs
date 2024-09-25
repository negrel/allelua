use std::{ffi::OsString, fmt::Display, ops::Deref, path::Path, process::exit};

use error::load_error;
use mlua::{chunk, AsChunk, FromLuaMulti, Lua, LuaOptions, StdLib};

use self::{
    byte::load_byte, env::load_env, globals::register_globals, os::load_os, package::load_package,
    path::load_path, string::load_string, sync::load_sync, table::load_table, test::load_test,
    time::load_time,
};

mod byte;
mod env;
mod error;
mod globals;
mod io;
mod os;
mod package;
mod path;
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
            | StdLib::STRING;

        if safety == RuntimeSafetyLevel::Unsafe {
            stdlib = stdlib | StdLib::FFI | StdLib::JIT | StdLib::DEBUG
        }

        let vm = unsafe { Lua::unsafe_new_with(stdlib, LuaOptions::new()) };

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
    handle_result(load_byte(lua.clone()));
    handle_result(load_env(lua.clone(), run_args));
    handle_result(load_path(lua.clone()));
    handle_result(load_os(lua.clone()));
    handle_result(load_error(lua.clone()));
    handle_result(load_string(lua.clone()));
    handle_result(load_sync(lua.clone()));
    handle_result(load_table(lua.clone()));
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

            // Freeze modules.
            table.map(package.loaded, function(k, v)
                return table.freeze(v)
            end)
        })
        .eval::<()>();

    handle_result(result);
}
