use anyhow::bail;
use mlua::{chunk, ErrorContext};
use std::{
    env,
    path::{self, PathBuf},
};
use walkdir::WalkDir;

use crate::lua::Runtime;

fn is_dir_or_test_file(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file()
            && entry.file_name().as_encoded_bytes().ends_with(b"_test.lua"))
}

pub fn test(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    let paths = if paths.is_empty() {
        vec![env::current_dir()?]
    } else {
        paths
    };

    let mut all_test_suite_ok = true;

    for path in paths {
        let iter = WalkDir::new(path)
            .into_iter()
            .filter_entry(is_dir_or_test_file);

        for entry in iter {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }

            let fpath = path::absolute(entry.into_path())?;
            let runtime = Runtime::new(&fpath, vec![]);

            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the tokio Runtime")
                .block_on(async {
                    // Execute code.
                    let test_suite_ok = async {
                        runtime
                            .exec::<()>(fpath.clone())
                            .await
                            .with_context(|_| format!("failed to load lua test file {fpath:?}"))?;

                        runtime
                            .exec::<bool>(chunk! {
                                local test = require("test")
                                return test.__execute_test_suite()
                            })
                            .await
                            .with_context(|_| {
                                format!("failed to execute test suite of lua file {fpath:?}",)
                            })
                    }
                    .await?;

                    all_test_suite_ok &= test_suite_ok;

                    Ok::<_, anyhow::Error>(())
                })?;
        }
    }

    if !all_test_suite_ok {
        bail!("some test failed")
    }

    Ok(())
}
