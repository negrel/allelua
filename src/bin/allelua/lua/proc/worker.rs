use std::{
    ops::{Deref, DerefMut},
    os::fd::{FromRawFd, IntoRawFd},
    path::Path,
    process::Stdio,
};

use mlua::{IntoLua, Lua, MetaMethod, UserData};
use tokio::{
    process::{self, Command},
    sync::Mutex,
};

use crate::lua::{io, os::LuaFile};

/// Worker define a separate allelua process that can communicate with the
/// main process via stdin/stderr.
#[derive(Debug)]
struct Worker {
    child: process::Child,
    stdin: mlua::Value,
    stderr: mlua::Value,
}

impl Deref for Worker {
    type Target = process::Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl DerefMut for Worker {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

/// LuaWorker define a Lua wrapper around an allelua worker process.
#[derive(Debug)]
pub struct LuaWorker(Mutex<Worker>);

impl LuaWorker {
    pub fn new(lua: &Lua, fpath: &Path) -> mlua::Result<Self> {
        let mut child = Command::new(std::env::args().nth(0).unwrap_or("allelua".to_string()))
            .arg("worker")
            .arg(fpath)
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(io::LuaError::from)?;

        let stdin = unsafe {
            LuaFile::from_raw_fd(
                child
                    .stdin
                    .take()
                    .unwrap()
                    .into_owned_fd()
                    .unwrap()
                    .into_raw_fd(),
            )
        };
        let stderr = unsafe {
            LuaFile::from_raw_fd(
                child
                    .stderr
                    .take()
                    .unwrap()
                    .into_owned_fd()
                    .unwrap()
                    .into_raw_fd(),
            )
        };

        Ok(Self(Mutex::new(Worker {
            child,
            stdin: stdin.into_lua(lua)?,
            stderr: stderr.into_lua(lua)?,
        })))
    }
}

impl UserData for LuaWorker {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "proc.InternalWorker");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("input", |_, w, ()| async move {
            let w = w.0.lock().await;
            Ok(w.stdin.clone())
        });

        methods.add_async_method("output", |_, w, ()| async move {
            let w = w.0.lock().await;
            Ok(w.stderr.clone())
        });

        methods.add_async_method("terminate", |_, w, ()| async move {
            let mut w = w.0.lock().await;
            w.kill().await.map_err(io::LuaError::from)?;
            Ok(())
        });

        methods.add_meta_method(MetaMethod::ToString, |_, w, ()| {
            let address = w as *const _ as usize;
            Ok(format!("proc.Worker 0x{address:x}"))
        })
    }
}
