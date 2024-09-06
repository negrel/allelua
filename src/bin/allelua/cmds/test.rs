use anyhow::Context;
use mlua::chunk;
use std::{env, path::PathBuf};
use tokio::task;
use walkdir::WalkDir;

use crate::lua::Runtime;

fn is_dir_or_test_file(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file()
            && entry.file_name().as_encoded_bytes().ends_with(b"_test.lua"))
}

pub fn test(path: Option<PathBuf>) -> anyhow::Result<()> {
    let path = path.unwrap_or(env::current_dir()?);

    let iter = WalkDir::new(path)
        .into_iter()
        .filter_entry(is_dir_or_test_file);
    for entry in iter {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            continue;
        }

        let fpath = entry.into_path();
        let runtime = Runtime::new(&fpath, vec![]);

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed building the tokio Runtime")
            .block_on(async {
                // Execute code.
                let local = task::LocalSet::new();
                local
                    .run_until(async {
                        runtime
                            .exec::<()>(fpath.clone())
                            .await
                            .with_context(|| format!("failed to load lua test file {fpath:?}"))?;

                        runtime
                            .exec::<()>(chunk! {
                                local test = require("test")
                                test.__execute_suite()
                            })
                            .await
                            .with_context(|| {
                                format!("failed to execute test suite of lua file {fpath:?}",)
                            })
                    })
                    .await?;

                // Wait for background tasks.
                local.await;

                Ok::<_, anyhow::Error>(())
            })?;
    }
    Ok(())
}
