use std::ops::Deref;

use mlua::{chunk, Either, FromLua, IntoLua, Lua, MetaMethod, UserData};

use crate::include_lua;

pub fn load_string(lua: &Lua) -> mlua::Result<()> {
    let string_extra = lua.create_table()?;
    let slice = lua
        .load(chunk! {
            return require("string").sub
        })
        .eval::<mlua::Function>()?;

    string_extra.set(
        "split",
        lua.create_function(
            |lua, (str, sep): (mlua::String, Either<mlua::String, Regex>)| {
                let sep = regex_or_escaped_regex(sep)?;

                let result = lua.create_table()?;

                for part in sep.split(&str.as_bytes()) {
                    result.push(lua.create_string(part)?)?;
                }

                Ok(result)
            },
        )?,
    )?;

    string_extra.set(
        "splitn",
        lua.create_function(
            |lua, (str, sep, n): (mlua::String, Either<mlua::String, Regex>, usize)| {
                let regex = regex_or_escaped_regex(sep)?;

                let result = lua.create_table()?;

                for part in regex.splitn(&str.as_bytes(), n) {
                    result.push(lua.create_string(part)?)?;
                }

                Ok(result)
            },
        )?,
    )?;

    {
        let slice = slice.clone();
        string_extra.set(
            "find",
            lua.create_function(
                move |lua,
                      (str, pattern, at): (
                    mlua::String,
                    Either<mlua::String, Regex>,
                    Option<usize>,
                )| {
                    let pattern = regex_or_escaped_regex(pattern)?;

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
                Either<mlua::String, Regex>,
                mlua::String,
                Option<usize>,
            )| {
                let pattern = regex_or_escaped_regex(pattern)?;
                let replacement: &[u8] = &replacement.as_bytes();

                lua.create_string(pattern.replacen(&str.as_bytes(), n.unwrap_or(1), replacement))
            },
        )?,
    )?;

    string_extra.set(
        "replace_all",
        lua.create_function(
            |lua,
             (str, pattern, replacement): (
                mlua::String,
                Either<mlua::String, Regex>,
                mlua::String,
            )| {
                let pattern = regex_or_escaped_regex(pattern)?;
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
                move |lua,
                      (str, pattern, at): (
                    mlua::String,
                    Either<mlua::String, Regex>,
                    Option<usize>,
                )| {
                    let pattern = regex_or_escaped_regex(pattern)?;

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
        lua.create_function(|_lua, str: mlua::String| Ok(regex::escape(&str.to_str()?)))?,
    )?;
    regex_constructors.set(
        "new",
        lua.create_function(|_lua, str: mlua::String| {
            let str = str.to_str()?;
            let re = regex::bytes::Regex::new(&str).map_err(mlua::Error::external)?;
            Ok(Regex(re))
        })?,
    )?;

    let string_mt = lua
        .load(include_lua!("./string.lua"))
        .eval::<mlua::Function>()?
        .call::<mlua::Table>((regex_constructors, string_extra))?;

    lua.set_type_metatable::<mlua::String>(Some(string_mt));

    Ok(())
}

#[derive(Debug, Clone, FromLua)]
pub struct Regex(regex::bytes::Regex);

impl Deref for Regex {
    type Target = regex::bytes::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for Regex {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "Regex")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, re, ()| {
            Ok(format!("Regex({})", re.0.as_str()))
        });
    }
}

fn regex_or_escaped_regex(str_or_regex: Either<mlua::String, Regex>) -> mlua::Result<Regex> {
    match str_or_regex {
        Either::Left(str) => Ok(Regex(
            regex::bytes::Regex::new(&regex::escape(&str.to_str()?))
                .map_err(mlua::Error::external)?,
        )),
        Either::Right(re) => Ok(re),
    }
}
