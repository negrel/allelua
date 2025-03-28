use std::{env, fs, path::PathBuf};

use anyhow::{bail, Context};
use codespan_reporting::term::termcolor::{self};
use walkdir::WalkDir;

pub fn lint(paths: Vec<PathBuf>) -> anyhow::Result<()> {
    let paths = if paths.is_empty() {
        vec![env::current_dir()?]
    } else {
        paths
    };

    let mut file_count = 0;
    let mut problems = 0;

    let mut linter = lualint::Linter::new();

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

            match full_moon::parse(&source) {
                Ok(ast) => {
                    let mut diagnostics = linter.lint(&ast);
                    diagnostics.sort_by_key(|d| d.primary_label.range);

                    if !diagnostics.is_empty() {
                        eprintln!("FAILED");

                        for d in diagnostics {
                            let diag = d.into_codespan_diagnostic(source_id);
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
                Err(errors) => {
                    eprintln!("FAILED");
                    for err in errors {
                        let (start, end) = err.range();
                        let diag = codespan_reporting::diagnostic::Diagnostic {
                            severity: codespan_reporting::diagnostic::Severity::Error,
                            code: None,
                            message: err.error_message().to_string(),
                            labels: vec![codespan_reporting::diagnostic::Label {
                                style: codespan_reporting::diagnostic::LabelStyle::Primary,
                                file_id: source_id,
                                range: start.bytes()..end.bytes(),
                                message: "".to_owned(),
                            }],
                            notes: Vec::new(),
                        };

                        codespan_reporting::term::emit(
                            &mut stderr,
                            &codespan_reporting::term::Config::default(),
                            &files,
                            &diag,
                        )
                        .context("failed to report lint")?;
                        problems += 1;
                    }
                }
            };
        }
    }

    println!("Checked {file_count} files.");
    if problems > 0 {
        bail!("Found {problems} problems.");
    }

    Ok(())
}

fn is_dir_or_lua_file(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file() && entry.file_name().as_encoded_bytes().ends_with(b".lua"))
}
