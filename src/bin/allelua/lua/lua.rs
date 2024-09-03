use mlua::Lua;

async fn go(_lua: &Lua, func: mlua::Function<'static>) -> mlua::Result<()> {
    let fut = func.call_async::<_, ()>(());
    tokio::task::spawn_local(async {
        if let Err(err) = fut.await {
            panic!("{err}")
        }
    });

    Ok(())
}

pub fn register_globals(lua: &'static Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    globals.set("go", lua.create_async_function(go)?)?;
    globals.set(
        "tostring",
        lua.load(include_str!("./globals/tostring.lua"))
            .eval::<mlua::Function>()?,
    )?;
    Ok(())
}
