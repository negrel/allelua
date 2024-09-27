use std::{
    future::{poll_fn, Future},
    pin::Pin,
    sync::atomic::AtomicUsize,
    task::Poll,
};

use mlua::{AnyUserData, FromLua, IntoLua, Lua, MetaMethod, ObjectLike, UserData};
use nanorand::Rng;
use tokio::task::AbortHandle;

use crate::include_lua;

use super::{
    error::{AlleluaError, LuaError},
    sync::{BufferedQueue, ChannelReceiver, LuaChannelReceiver, UnbufferedQueue},
};

static GOROUTINE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug)]
pub struct LuaAbortHandle(AbortHandle, usize);

impl UserData for LuaAbortHandle {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_lua, abort| Ok(abort.1))
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |lua, abort, ()| {
            lua.create_string(format!("AbortHandle(id={})", abort.1))
        });

        methods.add_meta_method(MetaMethod::Call, |_lua, abort, ()| {
            abort.0.abort();
            Ok(())
        })
    }
}

async fn go(_lua: Lua, func: mlua::Function) -> mlua::Result<LuaAbortHandle> {
    let fut = func.call_async::<()>(());
    let goroutine_id = GOROUTINE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let handle = tokio::task::spawn_local(async move {
        if let Err(err) = fut.await {
            eprintln!("goroutine {goroutine_id}: {err}");
        }
    });

    Ok(LuaAbortHandle(handle.abort_handle(), goroutine_id))
}

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

pub fn register_globals(lua: Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    globals.set("go", lua.create_async_function(go)?)?;
    globals.set("select", lua.create_async_function(select)?)?;

    let rawtype = globals.get::<mlua::Function>("type")?;
    globals.set("rawtype", rawtype)?;

    globals.set(
        "type",
        lua.create_function(move |lua, value: mlua::Value| match value {
            mlua::Value::Nil
            | mlua::Value::Boolean(_)
            | mlua::Value::Integer(_)
            | mlua::Value::Number(_)
            | mlua::Value::String(_)
            | mlua::Value::LightUserData(_)
            | mlua::Value::Function(_)
            | mlua::Value::Thread(_) => value.type_name().into_lua(lua),
            mlua::Value::Error(_) => "error".into_lua(lua),
            mlua::Value::Table(ref t) => {
                let __type = t.get::<mlua::Value>("__type")?;
                match __type {
                    mlua::Value::Nil => value.type_name().into_lua(lua),
                    mlua::Value::String(_) => Ok(__type),
                    mlua::Value::Function(func) => func.call::<mlua::Value>(t),
                    _ => Err(mlua::Error::FromLuaConversionError {
                        from: value.type_name(),
                        to: "function",
                        message: None,
                    }),
                }
            }
            mlua::Value::UserData(udata) => udata
                .get::<mlua::Value>("__type")
                .or_else(|_| "userdata".into_lua(lua)),
        })?,
    )?;

    globals.set(
        "print",
        lua.create_function(move |lua, values: mlua::MultiValue| {
            let tostring = lua.globals().get::<mlua::Function>("tostring").unwrap();
            for v in values {
                let str = tostring.call::<String>(v)?;
                print!("{str} ");
            }
            println!();
            Ok(())
        })?,
    )?;

    let clone_not_impl_err = LuaError::from(LuaCloneError::NotImplemented);
    lua.load(include_lua!("./globals.lua"))
        .eval::<mlua::Function>()?
        .call::<()>((globals.clone(), clone_not_impl_err))?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum LuaCloneError {
    #[error("__clone metamethod is not implemented")]
    NotImplemented,
}

impl AlleluaError for LuaCloneError {
    fn type_name(&self) -> &'static str {
        "CloneError"
    }

    fn kind(&self) -> &'static str {
        match self {
            LuaCloneError::NotImplemented => "NotImplemented",
        }
    }
}
