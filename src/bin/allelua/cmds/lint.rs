use std::{env, fs, path::PathBuf};

use anyhow::{bail, Context};
use selene_lib::{standard_library::StandardLibrary, Checker, CheckerConfig};
use walkdir::WalkDir;

use super::is_dir_or_lua_file;

pub fn lint(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    let paths = if paths.is_empty() {
        vec![env::current_dir()?]
    } else {
        paths
    };

    let mut files = 0;
    let mut problems = 0;

    let checker = Checker::new(
        CheckerConfig::<serde_json::Value>::default(),
        StandardLibrary::default(),
    )
    .unwrap();

    for path in paths {
        let iter = WalkDir::new(path)
            .into_iter()
            .filter_entry(is_dir_or_lua_file);

        for entry in iter {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }

            files += 1;

            let fpath = entry.into_path();

            let source = fs::read_to_string(&fpath)
                .with_context(|| format!("failed to read lua file {fpath:?}"))?;

            eprint!("linting file {fpath:?} ... ");
            let ast = full_moon::parse(&source)
                .with_context(|| format!("failed to parse lua file {fpath:?}"))?;

            let diagnostics = checker.test_on(&ast);

            if !diagnostics.is_empty() {
                eprintln!("FAILED");
                for diag in diagnostics {
                    let diag = diag.diagnostic;
                    eprintln!("\t{:?} {} [{}]", fpath, diag.message, diag.code);
                    problems += 1;
                }
            } else {
                eprintln!("ok");
            }
        }
    }

    println!("Checked {files} files.");
    if problems > 0 {
        bail!("Found {problems} problems");
    }

    Ok(())
}
