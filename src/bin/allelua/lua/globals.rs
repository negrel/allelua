use mlua::{chunk, Lua};

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
        lua.load(chunk! {
            local string = require("string")
            local table = require("table")

            local rawtostring = tostring

            local tostring = nil

            local function tostring_table(value, opts)
                local space = opts.space <= 0 and "" or "\n" .. string.rep(" ", opts.space * opts.depth)
                local close = opts.space <= 0 and " }" or "\n" .. string.rep(" ", opts.space * (opts.depth - 1)) .. "}"

                local inner_opts = {
                    space = opts.space,
                    depth = opts.depth + 1,
                    __stringified = opts.__stringified
                }

                local items = {}
                for k, v in pairs(value) do
                    local kv = { space }
                    if type(k) == "string" or type(k) == "number" then
                        table.push(kv, k)
                    else
                        table.push(kv, "[", tostring(k, inner_opts), "]")
                    end
                    table.push(kv, " = ")

                    if type(v) == "string" then
                        table.push(kv, '"', v, '"')
                    else
                        table.push(kv, tostring(v, inner_opts))
                    end

                    table.push(items, table.concat(kv))
                end

                // empty table ?
                if #items == 0 then return "{}" end

                return "{ " .. table.concat(items, ", ") .. close
            end

            local function tostring_impl(value, opts)
                // Call metamethod if any.
                local v_mt = getmetatable(value)
                if type(v_mt) == "table" and v_mt.__tostring ~= nil then
                    return v_mt.__tostring(value, opts)
                end

                // Custom default tostring for table.
                if type(value) == "table" then
                    return tostring_table(value, opts)
                end

                return rawtostring(value)
            end

            // A custom to string function that pretty format table and support
            // recursive values.
            tostring = function(v, opts)
                opts = opts or {}
                opts.__stringified = opts.__stringified or {}
                local stringified = opts.__stringified

                opts.space = opts.space or 2
                opts.depth = opts.depth or 1

                if type(v) == "function" or v == nil then return rawtostring(v) end

                if stringified[v] then
                    stringified[v] = stringified[v] + 1
                    return rawtostring(v)
                end
                stringified[v] = 1

                local result = tostring_impl(v, opts)

                if stringified[v] ~= 1 then // recursive value
                    // prepend type and address to output so
                    return rawtostring(v) .. " " .. result
                end

                return result
            end

            return tostring
        })
            .eval::<mlua::Function>()?,
    )?;
    Ok(())
}
