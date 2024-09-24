use std::{
    future::{poll_fn, Future},
    pin::Pin,
    sync::atomic::AtomicUsize,
    task::Poll,
};

use mlua::{chunk, AnyUserData, FromLua, IntoLua, Lua, MetaMethod, ObjectLike, UserData};
use nanorand::Rng;
use tokio::task::AbortHandle;

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
    globals.set(
        "tostring",
        lua.load(chunk! {
            local string = require("string")
            local table = require("table")

            rawtostring = tostring

            local tostring = nil

            local function tostring_table(value, opts)
                local space = opts.space <= 0 and "" or "\n" .. string.rep(" ", opts.space * opts.depth)
                local close = opts.space <= 0 and " }" or "\n" .. string.rep(" ", opts.space * (opts.depth - 1)) .. "}"

                local inner_opts = {
                    space = opts.space,
                    depth = opts.depth + 1,
                    __stringified = opts.__stringified
                }

                local items = {}
                for k, v in pairs(value) do
                    local kv = { space }
                    if type(k) == "string" or type(k) == "number" then
                        table.push(kv, k)
                    else
                        table.push(kv, "[", tostring(k, inner_opts), "]")
                    end
                    table.push(kv, " = ")

                    if type(v) == "string" then
                        table.push(kv, '"', v, '"')
                    else
                        table.push(kv, tostring(v, inner_opts))
                    end

                    table.push(items, table.concat(kv))
                end

                // empty table ?
                if #items == 0 then return "{}" end

                return "{ " .. table.concat(items, ", ") .. close
            end

            local function tostring_impl(value, opts)
                // Call metamethod if any.
                local v_mt = getmetatable(value)
                if type(v_mt) == "table" and v_mt.__tostring ~= nil then
                    return v_mt.__tostring(value, opts)
                end

                // Custom default tostring for table.
                if type(value) == "table" then
                    return tostring_table(value, opts)
                end

                return rawtostring(value)
            end

            // A custom to string function that pretty format table and support
            // recursive values.
            tostring = function(v, opts)
                opts = opts or {}
                opts.__stringified = opts.__stringified or {}
                local stringified = opts.__stringified

                opts.space = opts.space or 2
                opts.depth = opts.depth or 1

                if type(v) == "function" or v == nil then return rawtostring(v) end

                if stringified[v] then
                    stringified[v] = stringified[v] + 1
                    return rawtostring(v)
                end
                stringified[v] = 1

                local result = tostring_impl(v, opts)

                if stringified[v] ~= 1 then // recursive value
                    // prepend type and address to output so
                    return rawtostring(v) .. " " .. result
                end

                return result
            end

            return tostring
        })
            .eval::<mlua::Function>()?,
    )?;

    let tostring = globals.get::<mlua::Function>("tostring").unwrap();
    globals.set(
        "print",
        lua.create_function(move |_lua, values: mlua::MultiValue| {
            for v in values {
                let str = tostring.call::<String>(v)?;
                print!("{str} ");
            }
            println!();
            Ok(())
        })?,
    )?;

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
        "pcall",
        lua.load(chunk! {
            local pcall = pcall
            local toluaerror = _G.package.loaded.error.__toluaerror
            return function(...)
                local results = {pcall(...)}
                if not results[1] then
                    local lua_err = toluaerror(results[2])
                    return false, lua_err or results[2]
                end
                return table.unpack(results)
            end
        })
        .eval::<mlua::Function>()?,
    )?;

    let clone_not_impl_err = LuaError::from(LuaCloneError::NotImplemented);
    globals.set(
        "clone",
        lua.load(chunk! {
            local clone_impl = function(v, opts)
                if rawtype(v) == "table" then
                    local meta = getmetatable(v)
                    if meta then
                        if rawtype(meta.__clone) == "function" then
                            return meta.__clone(v, opts)
                        else
                            return meta.__clone
                        end
                    end
                    return $clone_not_impl_err
                elseif rawtype(v) == "userdata" then
                    if v.__clone then
                        if rawtype(v.__clone) == "function" then
                            return v.__clone(v, opts)
                        else
                            return v.__clone
                        end
                    end
                    return $clone_not_impl_err
                end

                return v
            end

            return function(v, opts)
                local opts = opts or {}
                opts.deep = opts.deep or false
                opts.replace = opts.replace or {}
                local replace = opts.replace

                if replace[v] then
                    return replace[v]
                end

                return clone_impl(v, opts)
            end
        })
        .eval::<mlua::Function>()?,
    )?;

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
