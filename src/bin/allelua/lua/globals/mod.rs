use std::{
    future::{poll_fn, Future},
    pin::Pin,
    task::Poll,
};

use mlua::{AnyUserData, FromLua, IntoLua, Lua, ObjectLike, UserData, UserDataMetatable};
use nanorand::Rng;

use crate::include_lua;

use super::{
    error::{AlleluaError, LuaError},
    sync::{BufferedQueue, ChannelReceiver, LuaChannelReceiver, UnbufferedQueue},
};

mod repl;

pub use repl::*;

async fn select(lua: Lua, table: mlua::Table) -> mlua::Result<()> {
    let default_callback = table.get::<Option<mlua::Function>>("default")?;
    let has_default_branch = default_callback.is_some();

    let mut callbacks = Vec::new();
    let mut futures = Vec::new();

    let mut i = 0;
    for res in table.pairs::<mlua::Value, mlua::Function>() {
        let (value, callback) = res?;
        let ch = match AnyUserData::from_lua(value, &lua) {
            Ok(userdata) => match userdata.borrow::<LuaChannelReceiver<BufferedQueue>>() {
                Ok(ch) => ChannelReceiver::Buffered(ch.clone()),
                Err(_) => match userdata.borrow::<LuaChannelReceiver<UnbufferedQueue>>() {
                    Ok(ch) => ChannelReceiver::Unbuffered(ch.clone()),
                    Err(_) => continue,
                },
            },
            Err(_) => continue,
        };
        futures.push((i, async move { ch.recv().await }, false));
        callbacks.push(callback);
        i += 1;
    }

    let output = poll_fn(|cx| {
        let branches = futures.len();
        let start = nanorand::tls_rng().generate::<usize>() % branches;

        for i in 0..branches {
            let branch = (start + i) % branches;
            let (i, ref mut fut, ref mut disabled) = futures[branch];

            if *disabled {
                continue;
            }

            let fut = unsafe { Pin::new_unchecked(fut) };
            let out = Future::poll(fut, cx);
            let out = match out {
                std::task::Poll::Ready(out) => out,
                std::task::Poll::Pending => {
                    continue;
                }
            };

            *disabled = true;

            return Poll::Ready(Some((i, out)));
        }

        if has_default_branch {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    })
    .await;

    if let Some((i, (val, true))) = output {
        let callback = &callbacks[i];
        callback.call_async(val).await?
    } else if let Some(cb) = default_callback {
        // Yield so other goroutines have time to send value if select is in a
        // loop.
        tokio::task::yield_now().await;
        cb.call_async(()).await?
    }

    Ok(())
}

pub fn lua_type(lua: &Lua, value: &mlua::Value) -> mlua::Result<mlua::String> {
    match value {
        mlua::Value::Error(_) => lua.create_string("error"),
        mlua::Value::Table(ref t) => {
            let __type = t.get::<mlua::Value>("__type");
            match __type {
                Ok(mlua::Value::String(str)) => Ok(str),
                _ => lua.create_string("table"),
            }
        }
        mlua::Value::UserData(udata) => match udata.get::<mlua::String>("__type") {
            Ok(v) => Ok(v),
            _ => lua.create_string("userdata"),
        },
        _ => lua.create_string(value.type_name()),
    }
}

pub fn register_globals(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();

    let debug = globals.get::<mlua::Table>("debug")?;

    globals.set("traceback", debug.get::<mlua::Function>("traceback")?)?;
    globals.set(
        "__repl",
        lua.create_async_function(|lua, ()| async move { repl(&lua).await })?,
    )?;

    globals.set("select", lua.create_async_function(select)?)?;

    let rawtype = globals.get::<mlua::Function>("type")?;
    globals.set("rawtype", rawtype)?;

    globals.set(
        "type",
        lua.create_function(|lua, value: mlua::Value| lua_type(lua, &value))?,
    )?;

    globals.set(
        "__debug",
        lua.create_function(|_lua, value: mlua::Value| Ok(format!("{value:?}")))?,
    )?;

    globals.set(
        "print",
        lua.create_async_function(move |lua, values: mlua::MultiValue| async move {
            let tostring = lua.globals().get::<mlua::Function>("tostring").unwrap();
            for v in values {
                let str = tostring
                    .call_async::<mlua::String>(v)
                    .await?
                    .to_string_lossy();

                print!("{} ", str);
            }
            println!();
            Ok(())
        })?,
    )?;

    let get_metatable = globals.get::<mlua::Function>("getmetatable")?;
    globals.set(
        "__rawgetmetatable",
        lua.create_function(move |lua, v: mlua::Value| match v {
            mlua::Value::Nil
            | mlua::Value::Boolean(_)
            | mlua::Value::LightUserData(_)
            | mlua::Value::Integer(_)
            | mlua::Value::Number(_)
            | mlua::Value::String(_)
            | mlua::Value::Function(_)
            | mlua::Value::Thread(_)
            | mlua::Value::Error(_) => get_metatable.call(v),
            mlua::Value::Table(tab) => Ok(tab
                .metatable()
                .map(mlua::Value::Table)
                .unwrap_or(mlua::Value::Nil)),
            mlua::Value::UserData(udata) => match udata.metatable() {
                Ok(v) => LuaUserDataMetadataTable(v).into_lua(lua),
                Err(_) => Ok(mlua::Value::Nil),
            },
            mlua::Value::Other(..) => Ok(mlua::Value::Nil),
        })?,
    )?;

    let clone_err = LuaError::from(LuaCloneError);
    lua.load(include_lua!("./globals.lua"))
        .eval::<mlua::Function>()?
        .call::<()>((globals.clone(), clone_err))?;

    Ok(())
}

#[derive(Debug, Clone, FromLua)]
struct LuaUserDataMetadataTable(UserDataMetatable);

impl UserData for LuaUserDataMetadataTable {
    fn add_fields<F: mlua::UserDataFields<Self>>(_fields: &mut F) {}

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::Index, |_lua, mt, k: mlua::String| match k
            .to_str()
        {
            Ok(k) => mt.0.get(k),
            Err(_) => Ok(mlua::Value::Nil),
        });

        methods.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |_lua, mt, (k, v): (mlua::String, mlua::Value)| {
                let k = k.to_str()?;
                mt.0.set(k, v)
            },
        );

        methods.add_meta_method(mlua::MetaMethod::Pairs, |lua, mt, ()| {
            let table = lua.create_table()?;
            for res in mt.0.pairs::<mlua::Value>() {
                let (k, v) = res?;
                table.set(k, v)?;
            }

            let next = lua.globals().get::<mlua::Function>("next")?;
            Ok((next, table))
        });
    }
}

#[derive(Debug, thiserror::Error)]
#[error("__clone metamethod is not implemented")]
struct LuaCloneError;

impl AlleluaError for LuaCloneError {
    fn type_name(&self) -> &'static str {
        "CloneError"
    }

    fn kind(&self) -> &'static str {
        "NotImplemented"
    }
}
