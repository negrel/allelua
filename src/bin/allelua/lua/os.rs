use std::process;

use mlua::Lua;

pub fn load_os(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "os",
        lua.create_function(|lua, ()| {
            let process = lua.create_table()?;

            process.set(
                "exit",
                lua.create_function(|_, code: i32| {
                    process::exit(code);
                    #[allow(unreachable_code)]
                    Ok(())
                })?,
            )?;

            Ok(process)
        })?,
    )
}
