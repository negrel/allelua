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
        fields.add_field("__type", "os.Child");
        fields.add_field_method_get("id", |_lua, child| Ok(child.id()));

        fields.add_field_method_get("stdin", |_lua, child| Ok(child.stdin.to_owned()));
        fields.add_field_method_get("stdout", |_lua, child| Ok(child.stdout.to_owned()));
        fields.add_field_method_get("stderr", |_lua, child| Ok(child.stderr.to_owned()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, child, ()| {
            let address = child as *const _ as usize;
            Ok(format!(
                "os.Child(id={}) 0x{address:x}",
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

async fn lua_str_or_obj_as_stdio(
    value: Either<mlua::String, mlua::AnyUserData>,
) -> mlua::Result<Stdio> {
    match value {
        Either::Left(str) => lua_string_as_stdio(str),
        Either::Right(obj) => lua_object_as_stdio(obj).await,
    }
}

pub async fn exec(
    lua: &Lua,
    (program, opts): (mlua::String, mlua::Table),
) -> mlua::Result<LuaChild> {
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
    if let Some(stdin) =
        opts.get::<Option<mlua::Either<mlua::String, mlua::AnyUserData>>>("stdin")?
    {
        let stdin = lua_str_or_obj_as_stdio(stdin)
            .await
            .with_context(|_| "invalid stdin option:")?;
        cmd.stdin(stdin);
    }

    // Handle stdout.
    if let Some(stdout) =
        opts.get::<Option<mlua::Either<mlua::String, mlua::AnyUserData>>>("stdout")?
    {
        let stdout = lua_str_or_obj_as_stdio(stdout)
            .await
            .with_context(|_| "invalid stdout option:")?;
        cmd.stdout(stdout);
    }

    // Handle stderr.
    if let Some(stderr) =
        opts.get::<Option<mlua::Either<mlua::String, mlua::AnyUserData>>>("stderr")?
    {
        let stderr = lua_str_or_obj_as_stdio(stderr)
            .await
            .with_context(|_| "invalid stderr option")?;
        cmd.stderr(stderr);
    }

    let mut child = cmd.spawn()?;

    let stdin = child
        .stdin
        .take()
        .map(LuaChildStdin::new)
        .transpose()?
        .map(|stdin| stdin.into_lua(lua))
        .unwrap_or(Ok(mlua::Value::Nil))?;
    let stdout = child
        .stdout
        .take()
        .map(LuaChildStdout::new)
        .transpose()?
        .map(|stdout| stdout.into_lua(lua))
        .unwrap_or(Ok(mlua::Value::Nil))?;
    let stderr = child
        .stderr
        .take()
        .map(LuaChildStderr::new)
        .transpose()?
        .map(|stderr| stderr.into_lua(lua))
        .unwrap_or(Ok(mlua::Value::Nil))?;

    let child = LuaChild::new(child, stdin, stdout, stderr);

    Ok(child)
}

#[derive(Debug)]
pub struct LuaExitStatus(ExitStatus);

impl UserData for LuaExitStatus {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.ExitStatus");
        fields.add_field_method_get("code", |_lua, status| Ok(status.0.code()));
        fields.add_field_method_get("success", |_lua, status| Ok(status.0.success()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, status, ()| {
            let address = status as *const _ as usize;
            Ok(format!(
                "os.ExitStatus(code={:?} success={}) 0x{address:x}",
                status.0.code(),
                status.0.success(),
            ))
        });
    }
}
