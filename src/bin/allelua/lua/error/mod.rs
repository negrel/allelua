use std::{ops::Deref, sync::Arc};

use mlua::{Either, FromLua, IntoLua, Lua, UserData, UserDataRef};

use crate::include_lua;

/// AlleluaError define common methods implemented by all errors returned by
/// std lib.
pub trait AlleluaError: std::error::Error + Send + Sync + 'static {
    fn type_name(&self) -> &str;
    fn kind(&self) -> &str;

    fn cause(&self) -> Option<LuaError> {
        None
    }

    fn field_getter(&self, _lua: &Lua, _key: mlua::String) -> mlua::Result<mlua::Value> {
        Ok(mlua::Nil)
    }
}

/// [UserError] define userdata returned by error.new() and error() and implements
/// [AlleluaError]
#[derive(Debug, thiserror::Error)]
#[error("{type_name}(kind={kind} message={message:?} cause={:?})", cause.as_ref().map(|v| v.to_string()))]
pub struct UserError {
    type_name: String,
    kind: String,
    message: String,
    #[source]
    cause: Option<LuaError>,
}

impl AlleluaError for UserError {
    fn type_name(&self) -> &str {
        self.type_name.as_str()
    }

    fn kind(&self) -> &str {
        self.kind.as_str()
    }

    fn cause(&self) -> Option<LuaError> {
        self.cause.clone()
    }
}

/// LuaError define a wrapper that implements [mlua::UserData] for the wrapped
/// [AlleluaError] type.
#[derive(thiserror::Error, Clone)]
#[error(transparent)]
pub struct LuaError(Arc<dyn AlleluaError>);

impl FromLua for LuaError {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        match Either::<UserDataRef<Self>, mlua::String>::from_lua(value, lua)? {
            Either::Left(udata) => Ok(udata.to_owned()),
            Either::Right(str) => Ok(Self::from(str)),
        }
    }
}

impl From<mlua::String> for LuaError {
    fn from(value: mlua::String) -> Self {
        UserError {
            type_name: "error".to_string(),
            kind: "uncategorized".to_string(),
            message: value.to_string_lossy(),
            cause: None,
        }
        .into()
    }
}

impl Deref for LuaError {
    type Target = dyn AlleluaError;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: AlleluaError> From<T> for LuaError {
    fn from(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl From<LuaError> for mlua::Error {
    fn from(val: LuaError) -> Self {
        mlua::Error::external(val)
    }
}

impl std::fmt::Debug for LuaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LuaError").field(&self.0).finish()
    }
}

impl UserData for LuaError {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("__type", |lua, err| lua.create_string(err.0.type_name()));
        fields.add_field_method_get("kind", |lua, err| lua.create_string(err.0.kind()));
        fields.add_field_method_get("message", |lua, err| lua.create_string(err.0.to_string()));
        fields.add_field_method_get("cause", |_, err| Ok(AlleluaError::cause(&**err)));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("is", |_lua, err, ref target: LuaError| Ok(err.is(target)));

        methods.add_meta_method(mlua::MetaMethod::Index, |lua, err, k: mlua::String| {
            err.0.field_getter(lua, k)
        });

        methods.add_meta_method(mlua::MetaMethod::ToString, |_lua, err, ()| {
            Ok(err.to_string())
        })
    }
}

impl LuaError {
    fn is(&self, target: &LuaError) -> bool {
        let mut err = self.to_owned();
        loop {
            if std::ptr::addr_eq(
                std::sync::Arc::<_>::as_ptr(&self.0),
                std::sync::Arc::<_>::as_ptr(&target.0),
            ) {
                return true;
            }

            if err.type_name() == target.type_name() && err.kind() == target.kind() {
                // Handle uncategorized error.
                if err.kind() == "uncategorized" && err.to_string() == target.to_string() {
                    return true;
                }
            }

            if let Some(cause) = AlleluaError::cause(&*err) {
                err = cause;
                continue;
            }
            return false;
        }
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

pub fn load_error(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "error",
        lua.create_function(|lua, ()| {
            let error = lua.create_table()?;

            error.set("throw", lua.globals().get::<mlua::Function>("error")?)?;
            lua.globals().set("error", error.clone())?;

            error.set(
                "throw_new",
                lua.create_function(
                    move |_lua, (message, options): (mlua::String, Option<mlua::Table>)| {
                        let type_name = options
                            .as_ref()
                            .map(|opt| opt.get::<Option<mlua::String>>("type"))
                            .transpose()?
                            .flatten()
                            .map(|t| t.to_string_lossy())
                            .unwrap_or_else(|| "error".to_owned());

                        let kind = options
                            .as_ref()
                            .map(|opt| opt.get::<Option<mlua::String>>("kind"))
                            .transpose()?
                            .flatten()
                            .map(|t| t.to_string_lossy())
                            .unwrap_or_else(|| "uncategorized".to_owned());

                        let cause = options
                            .as_ref()
                            .map(|opt| opt.get::<Option<LuaError>>("cause"))
                            .transpose()?
                            .flatten();

                        Err::<(), _>(mlua::Error::external(LuaError::from(UserError {
                            type_name,
                            kind,
                            message: message.to_string_lossy(),
                            cause,
                        })))
                    },
                )?,
            )?;

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

            error.set(
                "is",
                lua.create_function(|_lua, (err, target): (LuaError, LuaError)| {
                    Ok(err.is(&target))
                })?,
            )?;

            lua.load(include_lua!("./error.lua"))
                .eval::<mlua::Function>()?
                .call::<()>(error.clone())?;

            Ok(error)
        })?,
    )
}
