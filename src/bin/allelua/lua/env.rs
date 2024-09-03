use std::{
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
