use std::process::exit;

use allelua::Runtime;

pub fn run(args: &[String]) {
    if args.len() < 2 {
        cli_error(1, "no Lua file provided");
    }

    let cmd = &args[1];
    match cmd.as_str() {
        "-h" | "--help" | "help" => usage(),
        fname => match std::fs::read_to_string(fname) {
            Ok(lua_code) => {
                let mut rt = Runtime::new(args[1..].iter().cloned())
                    .expect("failed to initialize Allelua runtime");

                if let Err(err) = rt.do_chunk(lua_code) {
                    match err {
                        allelua::Error::LuaError(error) => {
                            eprintln!("lua error: {error}");
                            exit(1);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("failed to read lua file '{cmd}': {err}");
                exit(1);
            }
        },
    }
}

fn cli_error(exit_code: i32, msg: impl AsRef<str>) {
    eprintln!("Error: {}", msg.as_ref());
    eprintln!();
    eprintln!("USAGE: allelua run [FLAGS...] FILE [ARGS...]");
    eprintln!();
    eprintln!("Run 'allelua run -h' for more informations");
    exit(exit_code);
}

fn usage() {
    eprintln!("allelua run - Run a Lua file.");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("   allelua run [FLAGS...] FILE [ARGS...]");
    eprintln!();
    eprintln!("FLAGS:");
    eprintln!("   -h, --help                   Print this menu.");
}
