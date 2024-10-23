use anyhow::{bail, Context};
use mlua::chunk;
use std::{
    env,
    path::{self, PathBuf},
};
use walkdir::WalkDir;

use crate::lua::{Runtime, RuntimeSafetyLevel};

fn is_dir_or_bench_file(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file()
            && entry
                .file_name()
                .as_encoded_bytes()
                .ends_with(b"_bench.lua"))
}

pub fn bench(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    let paths = if paths.is_empty() {
        vec![env::current_dir()?]
    } else {
        paths
    };

    let mut all_bench_suite_ok = true;

    println!(
        "OS: {} ({})",
        std::env::consts::OS,
        std::env::consts::FAMILY
    );
    println!("ARCH: {}", std::env::consts::ARCH);

    for path in paths {
        let iter = WalkDir::new(path)
            .into_iter()
            .filter_entry(is_dir_or_bench_file);

        for entry in iter {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }

            let fpath = path::absolute(entry.into_path())?;
            let runtime = Runtime::new(&fpath, vec![], RuntimeSafetyLevel::Unsafe);

            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the tokio Runtime")
                .block_on(async {
                    // Execute code.
                    let bench_suite_ok = async {
                        runtime
                            .exec::<()>(fpath.clone())
                            .await
                            .with_context(|| format!("failed to load lua bench file {fpath:?}"))?;

                        runtime
                            .exec::<bool>(chunk! {
                                local test = require("test")
                                return test.__execute_bench_suite()
                            })
                            .await
                            .with_context(|| {
                                format!("failed to execute bench suite of lua file {fpath:?}",)
                            })
                    }
                    .await?;

                    all_bench_suite_ok &= bench_suite_ok;

                    Ok::<_, anyhow::Error>(())
                })?;
        }
    }

    if !all_bench_suite_ok {
        bail!("some bench returned an error")
    }

    Ok(())
}
