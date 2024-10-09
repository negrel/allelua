use std::{
    ffi::OsStr,
    ops::{Deref, DerefMut},
    os::unix::ffi::OsStrExt,
    process::{ExitStatus, Stdio},
};

use mlua::{IntoLua, Lua, MetaMethod, UserData};
use tokio::{
    io::{BufReader, BufWriter},
    process::{self, Child, ChildStderr, ChildStdin, ChildStdout},
};

use crate::lua::{
    error::LuaError,
    io::{self, add_io_close_methods, add_io_read_methods, add_io_write_close_methods, Closable},
};

pub struct LuaChild {
    child: Child,
    stdin: mlua::Value,
    stdout: mlua::Value,
    stderr: mlua::Value,
}

impl Deref for LuaChild {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl DerefMut for LuaChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

impl LuaChild {
    pub fn new(child: Child) -> Self {
        Self {
            child,
            stdin: mlua::Value::Nil,
            stdout: mlua::Value::Nil,
            stderr: mlua::Value::Nil,
        }
    }
}

impl UserData for LuaChild {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "Child");
        fields.add_field_method_get("id", |_lua, child| Ok(child.id()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, child, ()| {
            let address = child as *const _ as usize;
            Ok(format!(
                "Child(id={}) 0x{address:x}",
                child.id().unwrap_or(0)
            ))
        });

        methods.add_method_mut("stdin", |lua, child, ()| {
            if child.stdin.is_nil() {
                match child.child.stdin {
                    Some(_) => {
                        let stdin = child.child.stdin.take().unwrap();
                        child.stdin =
                            LuaChildStdin(Closable::new(BufWriter::new(stdin))).into_lua(lua)?;
                        Ok(child.stdin.to_owned())
                    }
                    None => Ok(mlua::Value::Nil),
                }
            } else {
                Ok(child.stdin.to_owned())
            }
        });

        methods.add_method_mut("stdout", |lua, child, ()| {
            if child.stdout.is_nil() {
                match child.child.stdout {
                    Some(_) => {
                        let stdout = child.child.stdout.take().unwrap();
                        child.stdout =
                            LuaChildStdout(Closable::new(BufReader::new(stdout))).into_lua(lua)?;
                        Ok(child.stdout.to_owned())
                    }
                    None => Ok(mlua::Value::Nil),
                }
            } else {
                Ok(child.stdout.to_owned())
            }
        });

        methods.add_method_mut("stderr", |lua, child, ()| {
            if child.stderr.is_nil() {
                match child.child.stderr {
                    Some(_) => {
                        let stderr = child.child.stderr.take().unwrap();
                        child.stderr =
                            LuaChildStderr(Closable::new(BufReader::new(stderr))).into_lua(lua)?;
                        Ok(child.stderr.to_owned())
                    }
                    None => Ok(mlua::Value::Nil),
                }
            } else {
                Ok(child.stderr.to_owned())
            }
        });

        methods.add_async_method_mut("wait", |_lua, mut child, ()| async move {
            let status = child
                .wait()
                .await
                .map_err(io::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            Ok(LuaExitStatus(status))
        });

        methods.add_async_method_mut("kill", |_lua, mut child, ()| async move {
            child
                .kill()
                .await
                .map_err(io::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            Ok(())
        });
    }
}

pub fn exec(_lua: &Lua, (program, opts): (mlua::String, mlua::Table)) -> mlua::Result<LuaChild> {
    let mut cmd = process::Command::new(OsStr::from_bytes(&program.as_bytes()));

    // Add args.
    if let Some(args) = opts.get::<Option<mlua::Table>>("args")? {
        args.for_each(|_k: mlua::Integer, v: mlua::String| {
            cmd.arg(OsStr::from_bytes(&v.as_bytes()));
            Ok(())
        })?;
    }

    // Add env vars.
    if let Some(env) = opts.get::<Option<mlua::Table>>("env")? {
        env.for_each(|k: mlua::String, v: mlua::String| {
            cmd.env(
                OsStr::from_bytes(&k.as_bytes()),
                OsStr::from_bytes(&v.as_bytes()),
            );
            Ok(())
        })?;
    }

    // Set working directory
    if let Some(workdir) = opts.get::<Option<mlua::String>>("workdir")? {
        cmd.current_dir(OsStr::from_bytes(workdir.as_bytes().as_ref()));
    }

    // Handle stdin.
    if let Some(stdin) = opts.get::<Option<mlua::String>>("stdin")? {
        match stdin.as_bytes().as_ref() {
            b"inherit" => {
                cmd.stdin(Stdio::inherit());
            }
            b"piped" => {
                cmd.stdin(Stdio::piped());
            }
            b"null" => {
                cmd.stdin(Stdio::null());
            }
            _ => return Err(mlua::Error::runtime("invalid stdin variant")),
        }
    }

    // Handle stdout.
    if let Some(stdout) = opts.get::<Option<mlua::String>>("stdout")? {
        match stdout.as_bytes().as_ref() {
            b"inherit" => {
                cmd.stdout(Stdio::inherit());
            }
            b"piped" => {
                cmd.stdout(Stdio::piped());
            }
            b"null" => {
                cmd.stdout(Stdio::null());
            }
            _ => return Err(mlua::Error::runtime("invalid stdout variant")),
        }
    }

    // Handle stderr.
    if let Some(stderr) = opts.get::<Option<mlua::String>>("stderr")? {
        match stderr.as_bytes().as_ref() {
            b"inherit" => {
                cmd.stderr(Stdio::inherit());
            }
            b"piped" => {
                cmd.stderr(Stdio::piped());
            }
            b"null" => {
                cmd.stderr(Stdio::null());
            }
            _ => return Err(mlua::Error::runtime("invalid stderr variant")),
        }
    }

    let child = cmd
        .spawn()
        .map_err(io::LuaError::from)
        .map_err(LuaError::from)
        .map_err(mlua::Error::external)?;

    Ok(LuaChild::new(child))
}

#[derive(Debug)]
pub struct LuaChildStdin(Closable<BufWriter<ChildStdin>>);

impl AsRef<Closable<BufWriter<ChildStdin>>> for LuaChildStdin {
    fn as_ref(&self) -> &Closable<BufWriter<ChildStdin>> {
        &self.0
    }
}

impl AsMut<Closable<BufWriter<ChildStdin>>> for LuaChildStdin {
    fn as_mut(&mut self) -> &mut Closable<BufWriter<ChildStdin>> {
        &mut self.0
    }
}

impl UserData for LuaChildStdin {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStdin");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_write_close_methods(methods);
    }
}

#[derive(Debug)]
pub struct LuaChildStdout(Closable<BufReader<ChildStdout>>);

impl AsRef<Closable<BufReader<ChildStdout>>> for LuaChildStdout {
    fn as_ref(&self) -> &Closable<BufReader<ChildStdout>> {
        &self.0
    }
}

impl AsMut<Closable<BufReader<ChildStdout>>> for LuaChildStdout {
    fn as_mut(&mut self) -> &mut Closable<BufReader<ChildStdout>> {
        &mut self.0
    }
}

impl UserData for LuaChildStdout {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStdout");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_close_methods(methods);
        add_io_read_methods(methods);
    }
}

#[derive(Debug)]
pub struct LuaChildStderr(Closable<BufReader<ChildStderr>>);

impl AsRef<Closable<BufReader<ChildStderr>>> for LuaChildStderr {
    fn as_ref(&self) -> &Closable<BufReader<ChildStderr>> {
        &self.0
    }
}

impl AsMut<Closable<BufReader<ChildStderr>>> for LuaChildStderr {
    fn as_mut(&mut self) -> &mut Closable<BufReader<ChildStderr>> {
        &mut self.0
    }
}

impl UserData for LuaChildStderr {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStderr");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_close_methods(methods);
        add_io_read_methods(methods);
    }
}

#[derive(Debug)]
pub struct LuaExitStatus(ExitStatus);

impl UserData for LuaExitStatus {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ExitStatus");
        fields.add_field_method_get("code", |_lua, status| Ok(status.0.code()));
        fields.add_field_method_get("success", |_lua, status| Ok(status.0.success()));
    }
}
