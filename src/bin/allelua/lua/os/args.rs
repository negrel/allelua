use std::ffi::OsString;

use mlua::{Lua, MetaMethod, UserData};

#[derive(Debug)]
pub(super) struct Args(Vec<mlua::String>);

impl Args {
    pub fn new(lua: &Lua, args: Vec<OsString>) -> mlua::Result<Self> {
        Ok(Self(
            args.into_iter()
                .map(|arg| lua.create_string(arg.as_encoded_bytes()))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl UserData for Args {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "Args");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |_, args, i: usize| {
            let arg = args.0.get(i).map(|str| str.to_owned());
            Ok(arg)
        })
    }
}
