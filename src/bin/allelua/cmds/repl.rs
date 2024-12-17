use std::path::PathBuf;

use clap::crate_version;

use crate::lua::Runtime;

pub fn repl() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the tokio Runtime")
        .block_on(async {
            let fpath = PathBuf::from("/repl");
            let runtime = Runtime::new(&fpath, vec![]);

            println!("allelua {}", crate_version!());
            runtime
                .exec::<()>(mlua::chunk! {
                    __repl()
                })
                .await
                .expect("failed to start REPL");
        })
}
