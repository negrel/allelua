mod bench;
mod fmt;
mod lint;
mod lsp;
mod repl;
mod run;
mod test;
mod worker;

pub use bench::*;
pub use fmt::*;
pub use lint::*;
pub use lsp::*;
pub use repl::*;
pub use run::*;
pub use test::*;
pub use worker::*;

fn is_dir_or_lua_file(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file() && entry.file_name().as_encoded_bytes().ends_with(b".lua"))
}
