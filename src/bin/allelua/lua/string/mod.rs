use mlua::{chunk, IntoLua, Lua};

use crate::include_lua;

mod regex;

pub use regex::*;

pub fn load_string(lua: &Lua) -> mlua::Result<()> {
    let string_extra = lua.create_table()?;
    let slice = lua
        .load(chunk! {
            return require("string").sub
        })
        .eval::<mlua::Function>()?;

    string_extra.set(
        "split",
        lua.create_function(|lua, (str, sep): (mlua::String, LuaRegex)| {
            let result = lua.create_table()?;

            for part in sep.split(&str.as_bytes()) {
                result.push(lua.create_string(part)?)?;
            }

            Ok(result)
        })?,
    )?;

    string_extra.set(
        "splitn",
        lua.create_function(|lua, (str, sep, n): (mlua::String, LuaRegex, usize)| {
            let result = lua.create_table()?;

            for part in sep.splitn(&str.as_bytes(), n) {
                result.push(lua.create_string(part)?)?;
            }

            Ok(result)
        })?,
    )?;

    {
        let slice = slice.clone();
        string_extra.set(
            "find",
            lua.create_function(
                move |lua, (str, pattern, at): (mlua::String, LuaRegex, Option<usize>)| {
                    match pattern.find_at(&str.as_bytes(), at.unwrap_or(0)) {
                        Some(m) => Ok((
                            slice.call::<mlua::Value>((&str, m.start() + 1, m.end()))?,
                            (m.start() + 1).into_lua(lua)?,
                            (m.end()).into_lua(lua)?,
                        )),
                        None => Ok((mlua::Value::Nil, mlua::Value::Nil, mlua::Value::Nil)),
                    }
                },
            )?,
        )?;
    }

    string_extra.set(
        "replace",
        lua.create_function(
            |lua,
             (str, pattern, replacement, n): (
                mlua::String,
                LuaRegex,
                mlua::String,
                Option<usize>,
            )| {
                let replacement: &[u8] = &replacement.as_bytes();

                lua.create_string(pattern.replacen(&str.as_bytes(), n.unwrap_or(1), replacement))
            },
        )?,
    )?;

    string_extra.set(
        "replace_all",
        lua.create_function(
            |lua, (str, pattern, replacement): (mlua::String, LuaRegex, mlua::String)| {
                let replacement: &[u8] = &replacement.as_bytes();

                lua.create_string(pattern.replace_all(&str.as_bytes(), replacement))
            },
        )?,
    )?;

    {
        let slice = slice.clone();
        string_extra.set(
            "captures",
            lua.create_function(
                move |lua, (str, pattern, at): (mlua::String, LuaRegex, Option<usize>)| {
                    match pattern.captures_at(&str.as_bytes(), at.unwrap_or(0)) {
                        Some(capture) => {
                            let result = lua.create_table()?;
                            let mut iter = pattern.capture_names().enumerate();
                            iter.next();

                            for (i, name) in iter {
                                let m = match name {
                                    Some(name) => capture.name(name),
                                    None => capture.get(i),
                                };

                                if let Some(m) = m {
                                    let start = m.start() + 1;
                                    let end = m.end();
                                    let substr = slice.call::<mlua::Value>((
                                        &str,
                                        m.start() + 1,
                                        m.end(),
                                    ))?;

                                    let tab = lua.create_table()?;
                                    tab.push(&substr)?;
                                    tab.push(start)?;
                                    tab.push(end)?;
                                    tab.push(name)?;

                                    tab.set("match", substr)?;
                                    tab.set("start", start)?;
                                    tab.set("end", end)?;
                                    tab.set("name", name)?;

                                    result.push(&tab)?;
                                    result.set(name, tab)?;
                                }
                            }

                            result.into_lua(lua)
                        }
                        None => Ok(mlua::Value::Nil),
                    }
                },
            )?,
        )?;
    }

    let regex_constructors = lua.create_table()?;
    regex_constructors.set(
        "escape",
        lua.create_function(|_lua, str: mlua::String| Ok(::regex::escape(&str.to_str()?)))?,
    )?;
    regex_constructors.set(
        "new",
        lua.create_function(|_lua, str: mlua::String| {
            LuaRegex::new(&str.to_str()?).map_err(mlua::Error::external)
        })?,
    )?;

    let string_mt = lua
        .load(include_lua!("./string.lua"))
        .eval::<mlua::Function>()?
        .call::<mlua::Table>((regex_constructors, string_extra))?;

    lua.set_type_metatable::<mlua::String>(Some(string_mt));

    Ok(())
}
