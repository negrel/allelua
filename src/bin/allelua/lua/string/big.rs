use std::{
    ops::{Deref, Range},
    rc::Rc,
};

use mlua::{FromLua, UserData};

use super::{
    lua_string_captures, lua_string_find, lua_string_replace, lua_string_replace_all,
    lua_string_split, lua_string_splitn, LuaRegex, LuaString, MaybeBig,
};

/// LuaBigString define a rust allocated string that can exceed Lua string size
/// limits.
#[derive(Debug, Clone, FromLua, PartialEq, Eq)]
pub struct LuaBigString {
    rc: Rc<[u8]>,
    range: Range<usize>,
}

impl Deref for LuaBigString {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.rc[self.range.clone()]
    }
}

impl<'a> LuaString<'a> for LuaBigString {
    type Bytes = &'a [u8];

    fn as_bytes(&'a self) -> Self::Bytes {
        self
    }

    fn create_string(&self, _: &mlua::prelude::Lua, bytes: &[u8]) -> mlua::Result<Self> {
        Ok(Self::from(bytes))
    }
}

impl UserData for LuaBigString {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "string.Big");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        // Native lua string methods.
        methods.add_method("rep", |_, str, n: usize| Ok(Self::from(str.repeat(n))));
        // TODO: use locale to change case.
        methods.add_method("lower", |_, str, ()| {
            Ok(Self::from(str.to_ascii_lowercase()))
        });
        // TODO: use locale to change case.
        methods.add_method("upper", |_, str, ()| {
            Ok(Self::from(str.to_ascii_uppercase()))
        });
        methods.add_method("len", |_, str, ()| Ok(str.len()));
        methods.add_method("reverse", |_, str, ()| {
            let mut data = str.rc.deref().to_owned();
            data.reverse();
            Ok(Self::from(data))
        });

        // Allelua extensions.
        methods.add_method("slice", |_, str, (i, j): (isize, Option<isize>)| {
            str.lua_slice(i, j)
        });
        methods.add_method("has_prefix", |_, str, prefix: MaybeBig| {
            let prefix = prefix.as_bytes();
            if prefix.len() > str.len() {
                Ok(false)
            } else {
                Ok(str[0..prefix.len()] == *prefix)
            }
        });
        methods.add_method("has_suffix", |_, str, suffix: MaybeBig| {
            let suffix = suffix.as_bytes();
            if suffix.len() > str.len() {
                Ok(false)
            } else {
                Ok(str[str.len() - suffix.len()..] == *suffix)
            }
        });
        methods.add_method("eq", |_, str, rhs: MaybeBig| {
            Ok(*str.as_bytes() == *rhs.as_bytes())
        });
        methods.add_method("toregex", |_lua, str, escape: Option<bool>| {
            let re = String::from_utf8_lossy(str.as_bytes());
            if let Some(true) = escape {
                let re = regex::escape(&re);
                Ok(LuaRegex::new(&re).map_err(mlua::Error::external)?)
            } else {
                Ok(LuaRegex::new(&re).map_err(mlua::Error::external)?)
            }
        });
        methods.add_method("split", |lua, str, re: LuaRegex| {
            lua_string_split(lua, (str.to_owned(), re))
        });
        methods.add_method("splitn", |lua, str, (re, n): (LuaRegex, usize)| {
            lua_string_splitn(lua, (str.to_owned(), re, n))
        });
        methods.add_method("find", |lua, str, (re, at): (LuaRegex, Option<usize>)| {
            lua_string_find(lua, str.to_owned(), re, at, |str, start, end| {
                str.lua_slice(
                    start.try_into().map_err(mlua::Error::runtime)?,
                    Some(end.try_into().map_err(mlua::Error::runtime)?),
                )
            })
        });
        methods.add_method(
            "replace",
            |lua, str, (re, replace, n): (LuaRegex, MaybeBig, Option<usize>)| {
                lua_string_replace(lua, (str.to_owned(), re, replace, n))
            },
        );
        methods.add_method(
            "replace_all",
            |lua, str, (re, replace): (LuaRegex, MaybeBig)| {
                lua_string_replace_all(lua, (str.to_owned(), re, replace))
            },
        );
        methods.add_method(
            "captures",
            |lua, str, (re, at): (LuaRegex, Option<usize>)| {
                lua_string_captures(lua, str.to_owned(), re, at, |str, start, end| {
                    str.lua_slice(
                        start.try_into().map_err(mlua::Error::runtime)?,
                        Some(end.try_into().map_err(mlua::Error::runtime)?),
                    )
                })
            },
        );

        methods.add_meta_method(mlua::MetaMethod::ToString, |lua, str, ()| {
            // Truncate string if it exceed max lua string size.
            if str.len() > 2147483391 {
                lua.create_string(&str[..2147483391])
            } else {
                lua.create_string(str.deref())
            }
        });

        methods.add_meta_method(mlua::MetaMethod::Concat, |_, str, rhs: MaybeBig| {
            let bytes = [str.deref(), &rhs.as_bytes()].concat();

            Ok(LuaBigString::from(bytes))
        });

        methods.add_meta_method(mlua::MetaMethod::Len, |_, str, ()| Ok(str.len()));

        methods.add_meta_method(mlua::MetaMethod::Lt, |_, str, rhs: MaybeBig| {
            match str.deref().cmp(&rhs.as_bytes()) {
                std::cmp::Ordering::Less => Ok(true),
                _ => Ok(false),
            }
        });

        methods.add_meta_method(mlua::MetaMethod::Le, |_, str, rhs: MaybeBig| {
            match str.deref().cmp(&rhs.as_bytes()) {
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => Ok(true),
                _ => Ok(false),
            }
        });

        methods.add_meta_method(mlua::MetaMethod::Eq, |_, str, rhs: Self| {
            Ok(str.deref() == rhs.deref())
        });
    }
}

impl<T: AsRef<[u8]>> From<T> for LuaBigString {
    fn from(value: T) -> Self {
        let rc: Rc<[u8]> = Rc::from(value.as_ref());
        let range = 0..rc.len();
        Self { rc, range }
    }
}

impl LuaBigString {
    pub fn lua_slice(&self, mut i: isize, j: Option<isize>) -> mlua::Result<Self> {
        let ilen: isize = self.len().try_into().map_err(mlua::Error::runtime)?;
        let ilen = ilen + 1;
        let mut j = j.unwrap_or(-1);

        // Translate negative indices.
        if i < 0 {
            i += ilen;
        }
        if j < 0 {
            j += ilen;
        }

        // i is less than 1, correct it to 1 (beginning of string).
        let start = if i < 1 { 1 } else { i.unsigned_abs() };
        // j is greater than string length, correct it to string length.
        let end = if j >= ilen {
            self.len()
        } else {
            j.unsigned_abs()
        };

        // Empty slice.
        if start > end {
            return Ok(Self::from(""));
        }

        Ok(self.slice((start - 1)..end))
    }

    pub fn slice(&self, range: Range<usize>) -> Self {
        Self {
            rc: self.rc.to_owned(),
            range,
        }
    }
}
