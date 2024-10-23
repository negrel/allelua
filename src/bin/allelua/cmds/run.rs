use std::{
    ffi::OsString,
    path::{self, PathBuf},
};

use mlua::ErrorContext;

use crate::lua::{Runtime, RuntimeSafetyLevel};

pub fn run(fpath: PathBuf, run_args: Vec<OsString>) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the tokio Runtime")
        .block_on(async {
            let fpath = path::absolute(&fpath)?;
            let runtime = Runtime::new(&fpath, run_args, RuntimeSafetyLevel::Safe);

            // Execute code.
            runtime
                .exec(fpath.clone())
                .await
                .with_context(|_| format!("failed to run lua file {:?}", fpath))?;

            Ok(())
        })
}
