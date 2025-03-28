use std::path::PathBuf;

use clap::Parser;

mod lint;

/// Lua runtime and tools blessed by programming gods (🙏).
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
enum Command {
    /// Lint Lua files.
    Lint {
        /// Path of Lua files or directory containing Lua files.
        paths: Vec<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Command::parse();

    match args {
        Command::Lint { paths } => lint::lint(paths),
    }
}
