use std::path::PathBuf;

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
    },
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> anyhow::Result<()> {
    // RUSTFLAGS="--cfg tokio_unstable" cargo build
    // console_subscriber::init();

    let parse = Cli::parse();

    match parse.command {
        Command::Run { file } => cmds::run(file).await?,
    }

    Ok(())
}
