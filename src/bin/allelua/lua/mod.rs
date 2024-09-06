use std::{ffi::OsString, fmt::Display, ops::Deref, path::Path, process::exit};

use mlua::{chunk, AsChunk, FromLuaMulti, Lua, LuaOptions, StdLib};

use self::{
    byte::load_byte, env::load_env, fs::load_fs, globals::register_globals, os::load_os,
    package::load_package, path::load_path, string::load_string, sync::load_sync,
    table::load_table, test::load_test, time::load_time,
};

mod byte;
mod env;
mod fs;
mod globals;
mod os;
mod package;
mod path;
mod string;
mod sync;
mod table;
mod test;
mod time;

/// Runtime define ready to use Lua VM with the allelua std lib loaded.
pub struct Runtime(&'static Lua);

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
    type Target = &'static Lua;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Runtime {
    pub fn new(fpath: &Path, run_args: Vec<OsString>) -> Self {
        let vm = unsafe {
            Lua::unsafe_new_with(
                StdLib::NONE
                    | StdLib::MATH
                    | StdLib::TABLE
                    | StdLib::PACKAGE
                    | StdLib::STRING
                    | StdLib::DEBUG,
                LuaOptions::new(),
            )
            .into_static()
        };

        prepare_runtime(vm, fpath, run_args);

        Runtime(vm)
    }

    pub async fn exec<'a, T>(&self, chunk: impl AsChunk<'static, 'a>) -> mlua::Result<T>
    where
        T: FromLuaMulti<'a> + 'a,
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

fn prepare_runtime(lua: &'static Lua, fpath: &Path, run_args: Vec<OsString>) {
    // Load libraries.
    handle_result(load_byte(lua));
    handle_result(load_env(lua, run_args));
    handle_result(load_fs(lua));
    handle_result(load_path(lua));
    handle_result(load_os(lua));
    handle_result(load_string(lua));
    handle_result(load_sync(lua));
    handle_result(load_table(lua));
    handle_result(load_time(lua));
    handle_result(register_globals(lua));

    // Depends on other package.
    handle_result(load_test(lua));

    // overwrite require.
    handle_result(load_package(lua, fpath));

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
