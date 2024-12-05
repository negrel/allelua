use std::{cell::RefCell, env, ffi::OsStr, os::unix::ffi::OsStrExt};

use mlua::{FromLua, MetaMethod, UserData};

#[derive(Debug, Default)]
pub(super) struct EnvVars;

impl UserData for EnvVars {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.EnvVars")
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

        methods.add_meta_method(MetaMethod::ToString, |_, _, opts: Option<mlua::Table>| {
            let nspace = opts
                .clone()
                .map(|t| t.get::<mlua::Integer>("space"))
                .unwrap_or(Ok(2))?;
            let depth = opts
                .map(|t| t.get::<mlua::Integer>("depth"))
                .unwrap_or(Ok(1))?;

            let space = if nspace <= 0 {
                ""
            } else {
                &" ".repeat((depth * nspace) as usize)
            };
            let close = if nspace <= 0 {
                " }"
            } else {
                &("\n".to_owned() + &" ".repeat(((depth - 1) * nspace) as usize))
            };

            Ok(format!(
                "os.EnvVars{{\n{}{}}}",
                env::vars_os()
                    .map(|(name, value)| format!("{}{name:?} = {value:?}", space))
                    .collect::<Vec<_>>()
                    .join(",\n"),
                close
            ))
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
