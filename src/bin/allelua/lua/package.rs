use std::{os::unix::ffi::OsStrExt, path::Path};

use mlua::{chunk, Lua};

pub fn load_package(lua: &'static Lua, fpath: &Path) -> mlua::Result<()> {
    let fpath = lua.create_string(fpath.as_os_str().as_bytes())?;

    // Delete coroutine library.
    lua.globals()
        .set::<_, Option<mlua::Value>>("coroutine", None)?;

    lua.load(chunk! {
        local package = require("package")
        local table = require("table")
        local env = require("env")
        local path = require("path")

        local M = package

        // Remove coroutine, table.new and table.clear module.
        package.loaded.coroutine = nil
        package.loaded["table.new"] = nil
        package.loaded["table.clear"] = nil

        // Remove path and cpath in favor or home made searchers.
        package.path = ""
        package.cpath = ""

        // Add meta table.
        M.meta = table.freeze({ path = $fpath, main = true })

        local file_loaded = {} // file_searcher loaded cache table.
        local function file_searcher(modname)
            local fpath = modname
            if string.has_prefix(fpath, "@/") then // relative to current working dir.
                fpath = path.join(env.current_dir(), string.slice(fpath, 3))
            elseif path.is_relative(fpath) then // relative to current file.
                fpath = path.join(path.parent(M.meta.path), fpath)
            end


            if not path.exists(fpath) then
                return "\n\tno file " .. fpath .. " found"
            end

            fpath = path.absolute(fpath)

            return function()
                if file_loaded[fpath] then return file_loaded[fpath] end
                local result = dofile(fpath)
                file_loaded[fpath] = result
                return result
            end, fpath
        end

        package.loaders = {
            package.searchers[1], // Preload loader.
            file_searcher,
        }
        package.searchers = package.loaders
    })
    .eval::<()>()
}
