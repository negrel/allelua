use std::{path::PathBuf, process::exit};

use allelua_runtime as allelua;

pub fn run(file: PathBuf, args: Vec<String>) {
    match std::fs::read_to_string(&file) {
        Ok(lua_code) => {
            let mut rt = allelua::Runtime::new(args.into_iter())
                .expect("failed to initialize Allelua runtime");

            if let Err(err) = rt.do_chunk(lua_code) {
                match err {
                    allelua::Error::LuaError(error) => {
                        eprintln!("Lua error: {error}");
                        exit(1);
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("failed to read Lua file '{}': {err}", file.display());
            exit(1);
        }
    }
}
