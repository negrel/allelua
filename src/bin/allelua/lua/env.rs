use std::{
    cell::RefCell,
    env,
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
};

use mlua::Lua;

pub fn load_env(lua: &'static Lua, run_args: Vec<OsString>) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "env",
        lua.create_function(move |_, ()| {
            let env = lua.create_table()?;

            let args = lua.create_table()?;
            for arg in &run_args {
                args.push(lua.create_string(arg.as_bytes())?)?;
            }

            env.set("args", args)?;

            let vars = lua.create_table()?;
            let vars_mt = lua.create_table()?;
            vars_mt.set("__type", lua.create_string("Vars")?)?;
            vars_mt.set(
                "__index",
                lua.create_function(|_, (_, varname): (mlua::Table, mlua::String)| {
                    let var = env::var_os(OsStr::from_bytes(varname.as_bytes()));
                    match var {
                        Some(var) => Ok(Some(lua.create_string(var.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;
            vars_mt.set(
                "__newindex",
                lua.create_function(
                    |_, (_, name, value): (mlua::Table, mlua::String, mlua::String)| {
                        unsafe {
                            env::set_var(
                                OsStr::from_bytes(name.as_bytes()),
                                OsStr::from_bytes(value.as_bytes()),
                            );
                        }
                        Ok(())
                    },
                )?,
            )?;
            vars_mt.set(
                "__tostring",
                lua.create_function(|lua, (_, opts): (mlua::Table, mlua::Table)| {
                    let table = lua.create_table()?;
                    for (varname, varvalue) in env::vars_os() {
                        table.set(
                            lua.create_string(varname.as_bytes())?,
                            lua.create_string(varvalue.as_bytes())?,
                        )?;
                    }
                    let tostring = lua.globals().get::<_, mlua::Function>("tostring")?;
                    tostring.call::<_, mlua::String>((table, opts))
                })?,
            )?;

            vars_mt.set(
                "__pairs",
                lua.create_function(move |lua, _: mlua::Table| {
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
                })?,
            )?;
            vars_mt.set("__metatable", false)?;
            vars.set_metatable(Some(vars_mt));
            env.set("vars", vars)?;

            env.set(
                "current_dir",
                lua.create_function(|lua, ()| {
                    let path = env::current_dir().map_err(mlua::Error::runtime)?;
                    lua.create_string(path.as_os_str().as_bytes())
                })?,
            )?;

            Ok(env)
        })?,
    )
}
