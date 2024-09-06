use std::{
    ffi::OsString,
    path::{self, PathBuf},
};

use anyhow::Context;
use tokio::task;

use crate::lua::prepare_vm;

pub fn run(fpath: PathBuf, run_args: Vec<OsString>) -> anyhow::Result<()> {
    return tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async {
            let fpath = path::absolute(&fpath)?;
            let lua = prepare_vm(&fpath, run_args);

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
        });
}
