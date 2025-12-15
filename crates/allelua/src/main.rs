use std::process::exit;

mod run;

pub use run::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        cli_error(1, "no command provided");
    }

    let cmd = &args[1];
    match cmd.as_str() {
        "-h" | "--help" | "help" => usage(),
        "r" | "ru" | "rnu" | "run" => run(&args[1..]),
        _ => cli_error(1, format!("unknown command: '{cmd}'")),
    }
}

fn cli_error(exit_code: i32, msg: impl AsRef<str>) {
    eprintln!("Error: {}", msg.as_ref());
    eprintln!();
    eprintln!("USAGE: allelua [FLAGS...] COMMAND [ARGS...]");
    eprintln!();
    eprintln!("Run 'allelua -h' for more informations");
    exit(exit_code);
}

fn usage() {
    eprintln!("allelua - a Lua runtime blessed by programming gods.");
    eprintln!("Alexandre Negrel <alexandre@negrel.dev>");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("   allelua [FLAGS...] COMMAND [ARGS...]");
    eprintln!();
    eprintln!("FLAGS:");
    eprintln!("   -h, --help                   Print this menu.");
    eprintln!();
    eprintln!("COMMANDS:");
    eprintln!("   help                         Print this menu.");
    eprintln!("   help COMMAND                 Print command's help menu.");
    eprintln!("   run  FILE                    Run a Lua file.");
    eprintln!();
    eprintln!("Source code: https://github.com/negrel/allelua");
}
