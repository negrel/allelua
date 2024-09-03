use std::{ffi::OsString, fmt::Display, path::Path, process::exit};

use mlua::{chunk, Lua};
use package::load_package;
use path::load_path;

use self::{
    byte::load_byte, env::load_env, fs::load_fs, globals::register_globals, string::load_string,
    sync::load_sync, table::load_table, time::load_time,
};

mod byte;
mod env;
mod fs;
mod globals;
mod package;
mod path;
mod string;
mod sync;
mod table;
mod time;

fn handle_result<T, E: Display>(result: Result<T, E>) {
    match result {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{err}");
            exit(-1);
        }
    }
}

pub fn prepare_runtime(lua: &'static Lua, fpath: &Path, run_args: Vec<OsString>) {
    // Load libraries.
    handle_result(load_byte(lua));
    handle_result(load_env(lua, run_args));
    handle_result(load_fs(lua));
    handle_result(load_path(lua));
    handle_result(load_string(lua));
    handle_result(load_sync(lua));
    handle_result(load_table(lua));
    handle_result(load_time(lua));
    handle_result(register_globals(lua));

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
