use std::{collections::HashMap, ffi::c_void};

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

#[derive(Default)]
struct PrintState {
    set: HashMap<*const c_void, usize>,
    user_data_count: usize,
    table_count: usize,
}

impl PrintState {
    fn add_table(&mut self, table: &mlua::Table) -> usize {
        if let Some(i) = self.set.insert(table.to_pointer(), self.table_count) {
            i
        } else {
            self.table_count += 1;
            self.table_count - 1
        }
    }

    fn add_user_data(&mut self, ud: &mlua::AnyUserData) -> usize {
        if let Some(i) = self.set.insert(ud.to_pointer(), self.user_data_count) {
            i
        } else {
            self.user_data_count += 1;
            self.user_data_count - 1
        }
    }

    fn get(&self, v: &mlua::Value) -> Option<usize> {
        self.set.get(&v.to_pointer()).copied()
    }
}

fn print_table(table: mlua::Table, prefix: &str, state: &mut PrintState) -> mlua::Result<()> {
    let idx = state.add_table(&table);

    let inner_prefix = prefix.to_owned() + "  ";

    if table.get_metatable().is_none() && table.is_empty() {
        print!("<table> {idx} {{}}");
        return Ok(());
    }

    println!("<table> {idx} {{");
    if let Some(mt) = table.get_metatable() {
        print!("{inner_prefix}<metatable> = ");
        print_value(mlua::Value::Table(mt), &inner_prefix, state)?;
        println!(",");
    }

    for pair in table.pairs::<mlua::Value, mlua::Value>() {
        let (key, value) = pair?;
        print!("{inner_prefix}");
        print_value(key, &inner_prefix, state)?;
        print!(" = ");
        print_value(value, &inner_prefix, state)?;
        println!(",");
    }
    print!("{prefix}}}");
    Ok(())
}

fn print_value(value: mlua::Value, prefix: &str, state: &mut PrintState) -> mlua::Result<()> {
    if let Some(idx) = state.get(&value) {
        print!("<{}> {idx}", value.type_name());
        return Ok(());
    }

    match value {
        mlua::Value::Nil => print!("nil"),
        mlua::Value::Boolean(b) => print!("{b}"),
        mlua::Value::LightUserData(ud) => print!("0x{:x}", ud.0 as usize),
        mlua::Value::Integer(n) => print!("{n}"),
        mlua::Value::Number(n) => print!("{n}"),
        mlua::Value::String(str) => print!("{str:?}",),
        mlua::Value::Table(t) => print_table(t, prefix, state)?,
        mlua::Value::Function(f) => {
            let info = f.info();
            match info.what {
                "C" => print!("<function> C"),
                "Lua" => print!(
                    "<function> Lua {}:{}",
                    info.short_src.unwrap_or("".to_string()),
                    info.line_defined.unwrap_or(0),
                ),
                _ => print!("<function> Rust"),
            }
        }
        mlua::Value::Thread(_) => unreachable!(),
        mlua::Value::UserData(ref ud) => {
            let idx = state.add_user_data(ud);
            print!("<userdata> {idx} {}", value.to_string()?);
        }
        mlua::Value::Error(err) => print!("Error({err})"),
    }

    Ok(())
}

async fn print(_lua: &Lua, values: mlua::MultiValue<'_>) -> mlua::Result<()> {
    let mut state = PrintState::default();
    for value in values {
        if let mlua::Value::String(str) = value {
            let str = String::from_utf8_lossy(str.as_bytes_with_nul());
            print!("{}", str);
        } else {
            print_value(value, "", &mut state)?;
        }
        print!(" ");
    }
    println!();
    Ok(())
}

pub fn register_globals(lua: &'static Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    globals.set("go", lua.create_async_function(go)?)?;
    globals.set("print", lua.create_async_function(print)?)?;
    Ok(())
}
