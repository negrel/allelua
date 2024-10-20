use std::{
    ffi::OsStr,
    ops::{Deref, DerefMut},
    os::unix::ffi::OsStrExt,
    process::{ExitStatus, Stdio},
};

use mlua::{Either, ErrorContext, IntoLua, Lua, MetaMethod, ObjectLike, UserData};
use tokio::process::{self, Child};

mod stderr;
mod stdin;
mod stdout;

use stderr::*;
use stdin::*;
use stdout::*;

use super::LuaStdio;

#[derive(Debug)]
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
    pub fn new(child: Child, stdin: mlua::Value, stdout: mlua::Value, stderr: mlua::Value) -> Self {
        Self {
            child,
            stdin,
            stdout,
            stderr,
        }
    }
}

impl UserData for LuaChild {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "Child");
        fields.add_field_method_get("id", |_lua, child| Ok(child.id()));

        fields.add_field_method_get("stdin", |_lua, child| Ok(child.stdin.to_owned()));
        fields.add_field_method_get("stdout", |_lua, child| Ok(child.stdout.to_owned()));
        fields.add_field_method_get("stderr", |_lua, child| Ok(child.stderr.to_owned()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, child, ()| {
            let address = child as *const _ as usize;
            Ok(format!(
                "Child(id={}) 0x{address:x}",
                child.id().unwrap_or(0)
            ))
        });

        methods.add_async_method_mut("wait", |_lua, mut child, ()| async move {
            let status = child.wait().await?;
            Ok(LuaExitStatus(status))
        });

        methods.add_async_method_mut("kill", |_lua, mut child, ()| async move {
            child.kill().await?;
            Ok(())
        });
    }
}

fn lua_string_as_stdio(str: mlua::String) -> mlua::Result<Stdio> {
    match str.as_bytes().as_ref() {
        b"inherit" => Ok(Stdio::inherit()),
        b"piped" => Ok(Stdio::piped()),
        b"null" => Ok(Stdio::null()),
        _ => Err(mlua::Error::external("invalid stdio")),
    }
}

async fn lua_object_as_stdio<T: ObjectLike + IntoLua>(obj: T) -> mlua::Result<Stdio> {
    let stdio = obj
        .call_async_method::<LuaStdio>("try_into_stdio", ())
        .await?;
    Ok(stdio.into())
}

async fn lua_value_as_stdio(
    value: Either<mlua::String, Either<mlua::AnyUserData, mlua::Table>>,
) -> mlua::Result<(Stdio, Option<usize>)> {
    match value {
        Either::Left(str) => lua_string_as_stdio(str).map(|stdio| (stdio, None)),
        Either::Right(obj) => match obj {
            Either::Left(udata) => lua_object_as_stdio(udata).await.map(|stdio| (stdio, None)),
            Either::Right(table) => match table.get::<Option<usize>>("buffer_size")? {
                Some(size) => {
                    match table.get::<Option<Either<mlua::String, mlua::AnyUserData>>>("from")? {
                        Some(Either::Right(udata)) => {
                            Ok((lua_object_as_stdio(udata).await?, Some(size)))
                        }
                        Some(Either::Left(str)) => Ok((lua_string_as_stdio(str)?, Some(size))),
                        None => Ok((Stdio::piped(), Some(size))),
                    }
                }
                None => Ok((lua_object_as_stdio(table).await?, None)),
            },
        },
    }
}

pub async fn exec(
    lua: &Lua,
    (program, opts): (mlua::String, mlua::Table),
) -> mlua::Result<LuaChild> {
    let mut cmd = process::Command::new(OsStr::from_bytes(&program.as_bytes()));
    let mut child_stdin_buffer_size = None;
    let mut child_stdout_buffer_size = None;
    let mut child_stderr_buffer_size = None;

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
    if let Some(stdin) =
        opts.get::<Option<Either<mlua::String, Either<mlua::AnyUserData, mlua::Table>>>>("stdin")?
    {
        let (stdin, buffer_size) = lua_value_as_stdio(stdin)
            .await
            .with_context(|_| "invalid stdin option:")?;
        cmd.stdin(stdin);
        child_stdin_buffer_size = buffer_size;
    }

    // Handle stdout.
    if let Some(stdout) =
        opts.get::<Option<Either<mlua::String, Either<mlua::AnyUserData, mlua::Table>>>>("stdout")?
    {
        let (stdout, buffer_size) = lua_value_as_stdio(stdout)
            .await
            .with_context(|_| "invalid stdout option:")?;

        cmd.stdout(stdout);
        child_stdout_buffer_size = buffer_size;
    }

    // Handle stderr.
    if let Some(stderr) =
        opts.get::<Option<Either<mlua::String, Either<mlua::AnyUserData, mlua::Table>>>>("stderr")?
    {
        let (stderr, buffer_size) = lua_value_as_stdio(stderr)
            .await
            .with_context(|_| "invalid stderr option")?;
        cmd.stderr(stderr);
        child_stderr_buffer_size = buffer_size;
    }

    let mut child = cmd.spawn()?;

    let (stdin, stdout, stderr) = {
        macro_rules! prepare_stdio {
            ($ident:ident, $buffer_size:ident, $new:expr, $new_buffered:expr) => {
                match (child.$ident.take(), $buffer_size) {
                    (Some($ident), Some(0)) => $new.into_lua(lua)?,
                    (Some($ident), Some(_)) | (Some($ident), None) => {
                        $new_buffered.into_lua(lua)?
                    }
                    _ => mlua::Value::Nil,
                }
            };
        }

        (
            prepare_stdio!(
                stdin,
                child_stdin_buffer_size,
                { LuaChildStdin::new(stdin) },
                { LuaChildStdin::new_buffered(stdin, child_stdin_buffer_size) }
            ),
            prepare_stdio!(
                stdout,
                child_stdout_buffer_size,
                { LuaChildStdout::new(stdout) },
                { LuaChildStdout::new_buffered(stdout, child_stdout_buffer_size) }
            ),
            prepare_stdio!(
                stderr,
                child_stderr_buffer_size,
                { LuaChildStderr::new(stderr) },
                { LuaChildStderr::new_buffered(stderr, child_stdin_buffer_size) }
            ),
        )
    };

    let child = LuaChild::new(child, stdin, stdout, stderr);

    Ok(child)
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
