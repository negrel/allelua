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
    Run {
        /// Path of file to run.
        file: PathBuf,
        /// Arguments passed to Lua program.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        run_args: Vec<OsString>,
    },

    Test {
        /// Path of test file or directory containing test files.
        path: Option<PathBuf>,
    },

    Bench {
        /// Path of bench file or directory containing bench files.
        path: Option<PathBuf>,
    },

    /// Format a lua file.
    Fmt {
        /// Path of lua file or directory containing lua files.
        path: Option<PathBuf>,

        #[arg(long)]
        check: bool,
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
    }

    Ok(())
}
