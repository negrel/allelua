use std::{
    ffi::OsString,
    path::{self, PathBuf},
};

use anyhow::Context;
use mlua::{Lua, LuaOptions, StdLib};
use tokio::task;

use crate::lua::prepare_runtime;

pub async fn run(fpath: PathBuf, run_args: Vec<OsString>) -> anyhow::Result<()> {
    let fpath = path::absolute(&fpath)?;

    // Create lua VM.
    let lua: &'static Lua = unsafe {
        Lua::unsafe_new_with(
            StdLib::NONE | StdLib::MATH | StdLib::TABLE | StdLib::PACKAGE | StdLib::STRING,
            LuaOptions::new(),
        )
        .into_static()
    };
    prepare_runtime(lua, &fpath, run_args);

    // Execute code.
    let local = task::LocalSet::new();
    local
        .run_until(lua.load(fpath.clone()).eval_async::<()>())
        .await
        .with_context(|| format!("failed to run lua file {:?}", fpath))?;

    // Wait for background tasks.
    local.await;

    // Collect everything so user data drop method get called (e.g. closing files).
    lua.gc_collect()?;
    lua.gc_collect()?;

    Ok(())
}
