use std::{ffi::OsString, path::PathBuf};

use clap::{Parser, Subcommand};

mod cmds;
mod lua;

/// Lua distribution blessed by the gods of programming.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// Run a Lua program.
    #[command(alias = "r")]
    Run {
        /// Path of file to run.
        file: PathBuf,
        /// Arguments passed to Lua program.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        run_args: Vec<OsString>,
    },

    /// Run tests using built-in test runner.
    #[command(alias = "t", alias = "tests")]
    Test {
        /// Path of test files or directory containing test files.
        path: Vec<PathBuf>,
    },

    /// Run benchmarks using built-in bench runner.
    #[command(alias = "b")]
    Bench {
        /// Path of bench files or directory containing bench files.
        path: Vec<PathBuf>,
    },

    /// Format lua files.
    #[command(alias = "f", alias = "format")]
    Fmt {
        /// Path of lua files or directory containing lua files.
        path: Vec<PathBuf>,

        #[arg(long)]
        check: bool,
    },

    /// Starts language server.
    Lsp,

    /// Lint lua files.
    Lint {
        /// Path of lua files or directory containing lua files.
        path: Vec<PathBuf>,
    },
}

pub fn main() -> anyhow::Result<()> {
    // RUSTFLAGS="--cfg tokio_unstable" cargo build
    // console_subscriber::init();

    match Cli::parse().command {
        Command::Run { file, run_args } => cmds::run(file, run_args)?,
        Command::Test { path } => cmds::test(path)?,
        Command::Bench { path } => cmds::bench(path)?,
        Command::Fmt { path, check } => cmds::fmt(path, check)?,
        Command::Lsp => cmds::lsp()?,
        Command::Lint { path } => cmds::lint(path)?,
    }

    Ok(())
}
