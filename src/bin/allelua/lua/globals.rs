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

pub fn register_globals(lua: &'static Lua, globals: &mlua::Table) -> mlua::Result<()> {
    globals.set("go", lua.create_async_function(go)?)?;
    Ok(())
}
