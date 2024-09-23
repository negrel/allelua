use std::sync::Arc;

use mlua::{chunk, IntoLua, Lua, UserData};

use thiserror::Error;

/// AlleluaError define common methods implemented by all errors returned by
/// std lib.
pub trait AlleluaError: std::error::Error + Send + Sync + 'static {
    fn type_name(&self) -> &'static str;
    fn kind(&self) -> &'static str;
}

/// LuaError define a wrapper around an [AlleluaError] type that implements
/// [mlua::UserData].
#[derive(Debug, Error, Clone)]
#[error("{0}")]
pub struct LuaError(Arc<dyn AlleluaError>);

impl<T: AlleluaError> From<T> for LuaError {
    fn from(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl UserData for LuaError {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("__type", |lua, err| lua.create_string(err.0.type_name()));
        fields.add_field_method_get("kind", |lua, err| lua.create_string(err.0.kind()));
        fields.add_field_method_get("message", |lua, err| lua.create_string(err.0.to_string()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_lua, err, ()| {
            Ok(format!(
                "{}(kind={} message={})",
                err.0.type_name(),
                err.0.kind(),
                err.0
            ))
        })
    }
}

fn to_lua_error(err: &mlua::Error) -> Option<LuaError> {
    match err {
        mlua::Error::CallbackError { cause, .. } => to_lua_error(cause),
        mlua::Error::ExternalError(err) => err
            .downcast_ref::<LuaError>()
            .map(|lua_err| lua_err.to_owned()),
        mlua::Error::WithContext { cause, .. } => to_lua_error(cause),
        _ => None,
    }
}

pub fn load_error(lua: Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "error",
        lua.create_function(|lua, ()| {
            let error = lua
                .load(chunk! {
                    local table = require("table")
                    local M = {}

                    return M
                })
                .eval::<mlua::Table>()?;

            error.set(
                "__toluaerror",
                lua.create_function(|lua, err: mlua::Error| {
                    if let Some(err) = to_lua_error(&err) {
                        let err = err.to_owned();
                        Ok(err.into_lua(lua)?)
                    } else {
                        Ok(mlua::Value::Nil)
                    }
                })?,
            )?;

            Ok(error)
        })?,
    )
}
