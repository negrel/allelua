use mlua::{chunk, Lua};

#[derive(Clone, mlua::FromLua)]
pub struct LuaStringModule;

impl mlua::UserData for LuaStringModule {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(#[allow(unused)] fields: &mut F) {
        for i in 0..100_000 {
            fields.add_field(i.to_string(), i)
        }
    }

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(#[allow(unused)] methods: &mut M) {
        let tostr = stringify!(LuaStringModule).replace("Lua", "");
        methods.add_meta_method(mlua::MetaMethod::ToString, move |_, _, ()| {
            Ok(tostr.to_owned())
        });

        methods.add_function("noop", |_, _v: mlua::Value| Ok(()))
    }
}

pub fn load_string(lua: &'static Lua) -> mlua::Result<()> {
    lua.load(chunk! {
        local string = require("string");
        local M = string

        M.slice = M.sub
        M.sub = nil

        M.has_prefix = function(str, prefix)
            return string.slice(str, 0, #prefix) == prefix
        end

        M.has_suffix = function(str, suffix)
            return string.slice(str, -#suffix) == suffix
        end
    })
    .eval::<()>()
}
