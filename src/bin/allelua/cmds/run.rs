use std::{
    ffi::OsString,
    path::{self, PathBuf},
};

use anyhow::Context;
use tokio::task;

use crate::lua::Runtime;

pub fn run(fpath: PathBuf, run_args: Vec<OsString>) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the tokio Runtime")
        .block_on(async {
            let fpath = path::absolute(&fpath)?;
            let lua = Runtime::new(&fpath, run_args);

            // Execute code.
            let local = task::LocalSet::new();
            local
                .run_until(lua.load(fpath.clone()).eval_async::<()>())
                .await
                .with_context(|| format!("failed to run lua file {:?}", fpath))?;

            // Wait for background tasks.
            local.await;

            Ok(())
        })
}
