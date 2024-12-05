use mlua::{IntoLua, Lua, LuaSerdeExt};

use crate::lua::error::AlleluaError;

use super::error;

pub fn load_json(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "json",
        lua.create_function(|lua, ()| {
            let json = lua.create_table()?;

            json.set("array_metatable", lua.array_metatable())?;
            json.set("null", lua.null())?;

            json.set(
                "encode",
                lua.create_function(
                    |_lua, (value, options): (mlua::Value, Option<mlua::Table>)| {
                        if let Some(Ok(true)) = options.map(|t| t.get::<bool>("pretty")) {
                            Ok(serde_json::to_string_pretty(&value).map_err(LuaError)?)
                        } else {
                            Ok(serde_json::to_string(&value).map_err(LuaError)?)
                        }
                    },
                )?,
            )?;

            json.set(
                "decode",
                lua.create_function(|lua, json: mlua::String| {
                    let value = serde_json::from_slice::<serde_json::Value>(&json.as_bytes())
                        .map_err(LuaError)?;

                    let mut err: Option<mlua::Error> = None;
                    let lua_value = json_value_to_lua_value(lua, value, &mut err);
                    if let Some(err) = err {
                        Err(err)
                    } else {
                        Ok(lua_value)
                    }
                })?,
            )?;

            Ok(json)
        })?,
    )
}

fn json_value_to_lua_value(
    lua: &Lua,
    value: serde_json::Value,
    err_ref: &mut Option<mlua::Error>,
) -> mlua::Value {
    macro_rules! if_no_err {
        ($err_ref:ident, $block:block) => {
            if $err_ref.is_none() {
                Some($block)
            } else {
                None
            }
        };
    }

    macro_rules! handle_result {
        ($err_ref:ident, $v:expr) => {
            match $v {
                Ok(v) => v,
                Err(err) => {
                    if_no_err!($err_ref, { *$err_ref = Some(err) });
                    mlua::Value::Nil
                }
            }
        };
    }

    if err_ref.is_some() {
        return mlua::Value::Nil;
    }

    match value {
        serde_json::Value::Null => mlua::Value::Nil,
        serde_json::Value::Bool(b) => mlua::Value::Boolean(b),
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                mlua::Value::Integer(n.as_i64().unwrap() as mlua::Integer)
            } else if n.is_u64() {
                mlua::Value::Integer(n.as_u64().unwrap() as mlua::Integer)
            } else {
                mlua::Value::Number(n.as_f64().unwrap() as mlua::Number)
            }
        }
        serde_json::Value::String(str) => handle_result!(err_ref, str.into_lua(lua)),
        serde_json::Value::Array(vec) => {
            handle_result!(
                err_ref,
                lua.create_sequence_from(vec.into_iter().map_while(|v| {
                    if_no_err!(err_ref, { json_value_to_lua_value(lua, v, err_ref) })
                }))
                .map(mlua::Value::Table)
            )
        }
        serde_json::Value::Object(map) => {
            handle_result!(
                err_ref,
                lua.create_table_from(map.into_iter().map(|(k, v)| {
                    (
                        if_no_err!(err_ref, {
                            json_value_to_lua_value(lua, serde_json::Value::String(k), err_ref)
                        }),
                        if_no_err!(err_ref, { json_value_to_lua_value(lua, v, err_ref) }),
                    )
                }))
                .map(mlua::Value::Table)
            )
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("json.Error(kind={} line={} column={})", self.kind(), self.0.line(), self.0.column())]
struct LuaError(serde_json::Error);

impl From<LuaError> for mlua::Error {
    fn from(val: LuaError) -> Self {
        error::LuaError::from(val).into()
    }
}

impl AlleluaError for LuaError {
    fn type_name(&self) -> &str {
        "json.Error"
    }

    fn kind(&self) -> &str {
        match self.0.classify() {
            serde_json::error::Category::Io => "io",
            serde_json::error::Category::Syntax => "syntax",
            serde_json::error::Category::Data => "data",
            serde_json::error::Category::Eof => "eof",
        }
    }

    fn cause(&self) -> Option<super::error::LuaError> {
        None
    }

    fn field_getter(&self, lua: &Lua, key: mlua::String) -> mlua::Result<mlua::Value> {
        match key.as_bytes().as_ref() {
            b"line" => self.0.line().into_lua(lua),
            b"column" => self.0.column().into_lua(lua),
            _ => Ok(mlua::Value::Nil),
        }
    }
}

impl From<serde_json::Error> for LuaError {
    fn from(value: serde_json::Error) -> Self {
        Self(value)
    }
}
