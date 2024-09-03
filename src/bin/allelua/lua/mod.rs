use std::{ffi::OsString, path::Path};

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

pub fn prepare_runtime(lua: &'static Lua, fpath: &Path, run_args: Vec<OsString>) {
    // Load libraries.
    register_globals(lua).unwrap();
    load_byte(lua).unwrap();
    load_env(lua, run_args).unwrap();
    load_fs(lua).unwrap();
    load_path(lua).unwrap();
    load_string(lua).unwrap();
    load_sync(lua).unwrap();
    load_table(lua).unwrap();
    load_time(lua).unwrap();

    // overwrite require.
    load_package(lua, fpath).unwrap();

    lua.load(chunk! {
        local package = require("package")
        local table = require("table")

        // Freeze modules.
        table.map(package.loaded, function(k, v)
            if type(v) == "table" then
                return table.freeze(v)
            else
                return v
            end
        end)
    })
    .eval::<()>()
    .unwrap();
}
