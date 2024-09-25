use std::{env, fs, path::PathBuf};

use anyhow::{bail, Context};
use stylua_lib::{format_code, Config, LineEndings, QuoteStyle, SortRequiresConfig};
use walkdir::WalkDir;

fn is_dir_or_lua_file(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file() && entry.file_name().as_encoded_bytes().ends_with(b".lua"))
}

pub fn fmt(path: Option<PathBuf>, check: bool) -> anyhow::Result<()> {
    let path = path.unwrap_or(env::current_dir()?);

    let mut has_error = false;

    let cfg = Config {
        column_width: 80,
        line_endings: LineEndings::Unix,
        indent_type: stylua_lib::IndentType::Tabs,
        indent_width: 2,
        quote_style: QuoteStyle::AutoPreferDouble,
        call_parentheses: stylua_lib::CallParenType::NoSingleTable,
        collapse_simple_statement: stylua_lib::CollapseSimpleStatement::Always,
        sort_requires: SortRequiresConfig { enabled: true },
        ..Default::default()
    };

    let iter = WalkDir::new(path)
        .into_iter()
        .filter_entry(is_dir_or_lua_file);

    for entry in iter {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }

        let fpath = entry.into_path();

        let source = fs::read_to_string(fpath.clone())
            .with_context(|| format!("failed to read lua file {fpath:?}"))?;

        let formatted_source =
            format_code(&source, cfg, None, stylua_lib::OutputVerification::None)
                .with_context(|| format!("failed to format lua file {fpath:?}"))?;

        if !check {
            match fs::write(fpath.clone(), formatted_source) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("failed to write file {fpath:?}: {err}");
                    has_error = true;
                }
            }
        }
    }

    if has_error {
        bail!("failed to format one or more files");
    }

    Ok(())
}
