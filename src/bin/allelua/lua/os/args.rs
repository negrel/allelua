use std::ffi::OsString;

use mlua::{AnyUserData, IntoLua, Lua, MetaMethod, UserData, UserDataRef};

#[derive(Debug)]
pub(super) struct LuaArgs(Vec<mlua::String>);

impl LuaArgs {
    pub fn new(lua: &Lua, args: Vec<OsString>) -> mlua::Result<Self> {
        Ok(Self(
            args.into_iter()
                .map(|arg| lua.create_string(arg.as_encoded_bytes()))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl UserData for LuaArgs {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.Args");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, args, i: usize| {
            match args.0.get(i.wrapping_sub(1)) {
                Some(str) => lua.create_string(str.as_bytes())?.into_lua(lua),
                None => Ok(mlua::Value::Nil),
            }
        });

        let ipairs = |lua: &Lua, args: AnyUserData| {
            let iter = lua.create_function(|lua, (args, i): (UserDataRef<LuaArgs>, usize)| {
                match args.0.get(i) {
                    Some(str) => Ok((
                        mlua::Value::Integer((i + 1) as i64),
                        lua.create_string(str.as_bytes())?.into_lua(lua)?,
                    )),
                    None => Ok((mlua::Value::Nil, mlua::Value::Nil)),
                }
            })?;

            Ok((iter, args, 0))
        };
        methods.add_meta_function(MetaMethod::IPairs, ipairs);
        methods.add_meta_function(MetaMethod::Pairs, ipairs);

        methods.add_meta_method(MetaMethod::ToString, |_, args, ()| {
            Ok(format!(
                "os.Args{{ {} }}",
                args.0
                    .iter()
                    .map(|arg| format!("{:?}", arg.to_string_lossy()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
    }
}
