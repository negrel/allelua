use std::{ffi::OsString, ops::Deref, path::PathBuf, rc::Rc};

use anyhow::Context;
use mlua::{Lua, LuaOptions, StdLib};
use tokio::task;

use crate::lua::{load_byte, load_env, load_fs, load_sync, load_time, register_globals};

pub async fn run(fpath: PathBuf, run_args: Vec<OsString>) -> anyhow::Result<()> {
    // Read file.
    let code = Rc::new(
        std::fs::read(&fpath).with_context(|| format!("Failed to read lua file {:?}", fpath))?,
    );

    // Create lua VM.
    let lua: &'static Lua = unsafe {
        Lua::unsafe_new_with(StdLib::DEBUG | StdLib::PACKAGE, LuaOptions::new()).into_static()
    };
    let globals = lua.globals();

    // Load libraries.
    register_globals(lua, &globals).unwrap();
    load_byte(lua)?;
    load_env(lua, run_args)?;
    load_fs(lua)?;
    load_sync(lua)?;
    load_time(lua)?;

    // Execute code.
    let local = task::LocalSet::new();
    local
        .run_until(lua.load(code.deref()).eval_async::<()>())
        .await
        .with_context(|| format!("failed to run lua file {:?}", fpath))?;

    // Wait for background tasks.
    local.await;

    // Collect everything so user data drop method get called (e.g. closing files).
    lua.gc_collect()?;
    lua.gc_collect()?;

    Ok(())
}
