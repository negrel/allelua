use std::{env, fs, path::PathBuf};

use anyhow::{bail, Context};
use stylua_lib::{
    format_code, Config, LineEndings, OutputVerification, QuoteStyle, SortRequiresConfig,
};
use walkdir::WalkDir;

#[allow(deprecated)]
const CONFIG: Config = Config {
    column_width: 80,
    line_endings: LineEndings::Unix,
    indent_type: stylua_lib::IndentType::Tabs,
    indent_width: 2,
    quote_style: QuoteStyle::AutoPreferDouble,
    call_parentheses: stylua_lib::CallParenType::NoSingleTable,
    collapse_simple_statement: stylua_lib::CollapseSimpleStatement::Always,
    sort_requires: SortRequiresConfig { enabled: true },
    no_call_parentheses: false,
};

fn is_dir_or_lua_file(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file() && entry.file_name().as_encoded_bytes().ends_with(b".lua"))
}

pub fn fmt(path: Option<PathBuf>, check: bool) -> anyhow::Result<()> {
    let path = path.unwrap_or(env::current_dir()?);

    let mut has_error = false;

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

        if check {
            eprint!("checking file {fpath:?} ... ");
        } else {
            eprint!("formatting file {fpath:?} ... ");
        }

        let formatted_source =
            format_str(&source).with_context(|| format!("failed to format lua file {fpath:?}"))?;

        if check {
            let text_diff = similar::TextDiff::from_lines(&source, &formatted_source);
            // If there are no changes, return nothing
            if text_diff.ratio() == 1.0 {
                eprintln!("ok");
            } else {
                eprintln!("FAILED");
                eprintln!(
                    "{}",
                    text_diff.unified_diff().header("original", "formatted")
                );
                has_error = true;
            }
        } else {
            match fs::write(fpath.clone(), formatted_source) {
                Ok(_) => eprintln!("ok"),
                Err(err) => {
                    eprintln!("FAILED");
                    eprintln!("{err}");
                    has_error = true;
                }
            }
        }
    }

    if has_error {
        if check {
            bail!("failed to check one or more files");
        } else {
            bail!("failed to format one or more files");
        }
    }

    Ok(())
}

pub fn format_str(str: &str) -> Result<String, stylua_lib::Error> {
    format_code(str, CONFIG, None, OutputVerification::None)
}
