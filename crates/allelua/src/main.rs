use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod run;
use run::*;

/// Lua runtime blessed by programming gods.
#[derive(Debug, Parser)]
#[command(version)]
struct Allelua {
    #[command(subcommand)]
    subcommand: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a Lua file.
    Run {
        /// Lua file to execute.
        file: PathBuf,
        /// Arguments passed to Lua main function
        args: Vec<String>,
    },
}

fn main() {
    let cmd = Allelua::parse();

    match cmd.subcommand {
        Command::Run { file, args } => run(file, args),
    }
}
