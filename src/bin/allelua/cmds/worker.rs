use std::{
    io::Write,
    path::{self, PathBuf},
    process::exit,
};

use mlua::ErrorContext;

use crate::lua::Runtime;

pub fn worker(fpath: PathBuf) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the tokio Runtime")
        .block_on(async {
            let fpath = path::absolute(&fpath).unwrap();
            let runtime = Runtime::new_worker(&fpath);

            // Write single byte to tell parent process we're ready.
            let mut stderr = std::io::stderr();
            stderr.write_all(b"0").unwrap();
            stderr.flush().unwrap();

            // Execute code.
            // We handle error here as we don't want to write anything to stderr
            // as it is used by parent process to communicate with this worker.
            if let Err(err) = runtime
                .exec::<()>(fpath.clone())
                .await
                .with_context(|_| format!("failed to run lua file {:?}", fpath))
            {
                println!("{err}");
                exit(1);
            }
        })
}
