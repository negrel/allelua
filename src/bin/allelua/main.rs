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
    Run {
        /// Path of file to run.
        file: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
        run_args: Vec<OsString>,
    },
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> anyhow::Result<()> {
    // RUSTFLAGS="--cfg tokio_unstable" cargo build
    // console_subscriber::init();

    let parse = Cli::parse();

    match parse.command {
        Command::Run { file, run_args } => cmds::run(file, run_args).await?,
    }

    Ok(())
}
