use mlua::chunk;
use std::{env, path::PathBuf};
use tokio::task;
use walkdir::WalkDir;

use crate::lua::prepare_vm;

fn is_dir_or_test_file(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file()
            && entry.file_name().as_encoded_bytes().ends_with(b"_test.lua"))
}

pub fn test(path: Option<PathBuf>) -> anyhow::Result<()> {
    let path = path.unwrap_or(env::current_dir()?);

    WalkDir::new(path)
        .into_iter()
        .filter_entry(is_dir_or_test_file)
        .for_each(|entry: Result<walkdir::DirEntry, walkdir::Error>| {
            let entry = entry.unwrap();
            if entry.file_type().is_dir() {
                return;
            }

            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(async {
                    let fpath = entry.into_path();
                    let lua = prepare_vm(&fpath, vec![]);

                    // Execute code.
                    let local = task::LocalSet::new();
                    local
                        .run_until(async {
                            lua.load(fpath.clone()).eval_async::<()>().await.unwrap();
                            let test = lua
                                .load(chunk! {
                                    return require("test")
                                })
                                .eval::<mlua::Table>()
                                .unwrap();
                            let exec = test.get::<_, mlua::Function>("__execute_suite").unwrap();
                            exec.call_async::<(), ()>(()).await.unwrap()
                        })
                        .await;

                    // Wait for background tasks.
                    local.await;

                    // Collect everything so user data drop method get called (e.g. closing files).
                    lua.gc_collect().unwrap();
                    lua.gc_collect().unwrap();
                })
        });

    Ok(())
}
