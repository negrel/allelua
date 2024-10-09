use std::{cell::RefCell, env, ffi::OsStr, os::unix::ffi::OsStrExt};

use mlua::{FromLua, MetaMethod, UserData};

#[derive(Debug, Default)]
pub(super) struct EnvVars();

impl UserData for EnvVars {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "EnvVars")
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, _, varname: mlua::String| {
            let var = env::var_os(OsStr::from_bytes(&varname.as_bytes()));
            match var {
                Some(var) => Ok(Some(lua.create_string(var.as_bytes())?)),
                None => Ok(None),
            }
        });

        methods.add_meta_method(
            MetaMethod::NewIndex,
            |lua, _, (name, value): (mlua::String, mlua::Value)| {
                let name = name.as_bytes();
                let name = OsStr::from_bytes(&name);

                // Safety: This function is safe to call in single threaded
                // program.
                unsafe {
                    if value.is_nil() {
                        env::remove_var(name)
                    } else {
                        let value = mlua::String::from_lua(value, lua)?;
                        env::set_var(name, OsStr::from_bytes(&value.as_bytes()));
                    }
                }

                Ok(())
            },
        );

        methods.add_meta_method(MetaMethod::ToString, |lua, _, opts: mlua::Value| {
            let table = lua.create_table()?;
            for (varname, varvalue) in env::vars_os() {
                table.set(
                    lua.create_string(varname.as_bytes())?,
                    lua.create_string(varvalue.as_bytes())?,
                )?;
            }
            let tostring = lua.globals().get::<mlua::Function>("tostring")?;
            tostring.call::<mlua::String>((table, opts))
        });

        methods.add_meta_method(MetaMethod::Pairs, |lua, _, ()| {
            let iter = RefCell::new(env::vars_os());
            lua.create_function(move |lua, _: mlua::Value| {
                let mut iter = iter.borrow_mut();
                if let Some((varname, varvalue)) = iter.next() {
                    let k = lua.create_string(varname.as_bytes())?;
                    let v = lua.create_string(varvalue.as_bytes())?;
                    Ok((mlua::Value::String(k), mlua::Value::String(v)))
                } else {
                    Ok((mlua::Value::Nil, mlua::Value::Nil))
                }
            })
        });
    }
}
