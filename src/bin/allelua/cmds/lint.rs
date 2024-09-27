use std::{collections::HashMap, env, fs, path::PathBuf};

use anyhow::{bail, Context};
use codespan_reporting::diagnostic::Severity as CodespanSeverity;
use codespan_reporting::term::termcolor::{self};
use selene_lib::{
    lints::Severity, standard_library::StandardLibrary, Checker, CheckerConfig, RobloxStdSource,
};
use serde_json::{json, Value};
use walkdir::WalkDir;

use super::is_dir_or_lua_file;

pub fn lint_checker() -> Checker<Value> {
    Checker::new(
        CheckerConfig {
            config: HashMap::from([(
                "multiple_statements".to_owned(),
                json!({ "one_line_if": "allow" }),
            )]),
            lints: HashMap::default(),
            std: Some("allelua".to_owned()),
            exclude: Vec::default(),
            roblox_std_source: RobloxStdSource::default(),
        },
        StandardLibrary::default(),
    )
    .unwrap()
}

pub fn lint(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    let paths = if paths.is_empty() {
        vec![env::current_dir()?]
    } else {
        paths
    };

    let mut file_count = 0;
    let mut problems = 0;

    let checker = lint_checker();

    let mut stderr = termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto);

    for path in paths {
        let iter = WalkDir::new(path)
            .into_iter()
            .filter_entry(is_dir_or_lua_file);

        for entry in iter {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }

            file_count += 1;

            let fpath = entry.into_path();
            let mut files = codespan::Files::new();

            let source = fs::read_to_string(&fpath)
                .with_context(|| format!("failed to read lua file {fpath:?}"))?;

            let source_id = files.add(fpath.as_os_str(), &source);

            eprint!("linting file {fpath:?} ... ");
            let ast = full_moon::parse(&source)
                .with_context(|| format!("failed to parse lua file {fpath:?}"))?;

            let mut diagnostics = checker.test_on(&ast);
            diagnostics.sort_by_key(|d| d.diagnostic.start_position());

            if !diagnostics.is_empty() {
                eprintln!("FAILED");

                for d in diagnostics {
                    let diag = d.diagnostic.into_codespan_diagnostic(
                        source_id,
                        match d.severity {
                            Severity::Allow => continue,
                            Severity::Error => CodespanSeverity::Error,
                            Severity::Warning => CodespanSeverity::Warning,
                        },
                    );
                    codespan_reporting::term::emit(
                        &mut stderr,
                        &codespan_reporting::term::Config::default(),
                        &files,
                        &diag,
                    )
                    .context("failed to report lint")?;
                    problems += 1;
                }
            } else {
                eprintln!("ok");
            }
        }
    }

    println!("Checked {file_count} files.");
    if problems > 0 {
        bail!("Found {problems} problems.");
    }

    Ok(())
}
