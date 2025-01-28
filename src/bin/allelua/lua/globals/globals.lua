local rawgetmetatable = __rawgetmetatable

local function is_table_like(v)
	local t = rawtype(v)
	return t == "table" or t == "userdata"
end

local function pcall_impl()
	local table = require("table")
	local toluaerror = package.loaded.error.__toluaerror
	local pcall = pcall

	return function(...)
		local results = { pcall(...) }
		if not results[1] then
			local lua_err = toluaerror(results[2])
			return false, lua_err or results[2]
		end
		return table.unpack(results)
	end
end

local function xpcall_impl()
	return function(fn, err_handler, ...)
		local ok, v_or_err = pcall(fn, ...)
		if not ok then
			return err_handler(v_or_err)
		else
			return ok, v_or_err
		end
	end
end

local function tostring_impl()
	local string = require("string")
	local table = require("table")
	local path = require("path")
	rawtostring = tostring
	local rawtostring = rawtostring

	local tostring = nil

	local function tostring_pairs(value, buf, opts)
		local space = opts.space <= 0 and ""
			or "\n" .. string.rep(" ", opts.space * opts.depth)
		local close = opts.space <= 0 and " }"
			or "\n" .. string.rep(" ", opts.space * (opts.depth - 1)) .. "}"

		local inner_opts = {
			space = opts.space,
			depth = opts.depth + 1,
			_stringified = opts._stringified,
		}

		buf:put("{")
		for k, v in pairs(value) do
			buf:put(space)
			if rawtype(k) == "string" or rawtype(k) == "number" then
				buf:put(k)
			else
				buf:put("[", tostring(k, inner_opts), "]")
			end
			buf:put(" = ")

			if rawtype(v) == "string" then
				buf:putf("%q", v)
			else
				local v_str = tostring(v, inner_opts)
				local v_type = type(v)
				if
					v_type ~= "number"
					and v_type ~= "integer"
					and v_type ~= "boolean"
					and v_type ~= "string"
					and not v_str:has_prefix(v_type)
				then
					buf:put(v_type, " ")
				end
				buf:put(v_str)
			end

			buf:put(", ")
		end
		buf:put(close)
	end

	-- selene: allow(shadowing)
	local function tostring_impl(value, buf, opts)
		-- Call metamethod if any.
		local v_mt = rawgetmetatable(value)

		if is_table_like(v_mt) then
			if rawtype(v_mt.__tostring) == "function" then
				buf:put(v_mt.__tostring(value, opts))
				return
			end

			-- Custom default tostring for __pairs.
			if rawtype(v_mt.__pairs) == "function" then
				tostring_pairs(value, buf, opts)
				return
			end
		end

		-- Custom default tostring for table.
		if rawtype(value) == "table" then
			tostring_pairs(value, buf, opts)
			return
		end

		buf:put(rawtostring(value))
	end

	local buf = string.buffer.new()

	-- A custom to string function that pretty format table and support
	-- recursive values.
	tostring = function(v, opts)
		opts = opts or {}
		opts._stringified = opts._stringified or {}
		local stringified = opts._stringified

		opts.space = opts.space or 2
		opts.depth = opts.depth or 1

		if rawtype(v) == "function" or v == nil then return rawtostring(v) end

		if stringified[v] then
			stringified[v] = stringified[v] + 1
			return rawtostring(v)
		end
		stringified[v] = 1

		-- selene: allow(shadowing)
		local buf = buf
		if #buf > 0 then buf = string.buffer.new() end
		tostring_impl(v, buf, opts)
		local result = buf:get()
		if #buf > 4096 then
			buf:free()
		else
			buf:reset()
		end

		if stringified[v] ~= 1 then -- recursive value
			-- prepend type and address to output so
			return rawtostring(v) .. " " .. result
		end

		return result
	end

	return tostring
end

local function clone_impl()
	local clone = nil

	local clone_pairs = function(value, opts)
		local value_clone = {}
		opts.replace[value] = value_clone

		local mt = rawgetmetatable(value)
		local mt_clone = mt
		if mt then
			if opts.metatable.skip then
				mt_clone = nil
			elseif
				not opts.metatable.shallow and mt.__name == "LuaUserDataMetadataTable"
			then
				mt_clone = clone(mt, opts.metatable)
			end
		end

		for k, v in pairs(value) do
			if not opts.shallow then k = clone(k, opts) end
			if not opts.shallow then v = clone(v, opts) end
			value_clone[k] = v
		end
		setmetatable(value_clone, mt_clone)

		return value_clone
	end

	local clone_any = function(v, opts)
		local v_mt = rawgetmetatable(v)
		if is_table_like(v_mt) then
			-- clone metamethod is defined
			if rawtype(v_mt.__clone) == "function" then
				return v_mt.__clone(v, opts)
			end

			-- __pairs is defined, clone using iterator.
			if rawtype(v_mt.__pairs) == "function" then
				return clone_pairs(v, opts)
			end
		end

		-- clone table using __pairs.
		if rawtype(v) == "table" then return clone_pairs(v, opts) end

		return v
	end

	clone = function(v, opts)
		opts = opts or {}
		opts.shallow = opts.shallow or false
		opts.metatable = opts.metatable or {}
		opts.replace = opts.replace or {}
		local replace = opts.replace

		if replace[v] then return replace[v] end

		return clone_any(v, opts)
	end

	return clone
end

local function switch_impl(v, cases, default)
	local case = cases[v]
	if case then
		case()
	else
		if default then default() end
	end
end

local function freeze_impl()
	local table = require("table")

	-- Frozen object error.
	local FrozenObjectError = {
		__type = "FrozenObjectError",
		__tostring = function(t)
			if t.kind == "Set" then
				return "cannot set "
					.. tostring(t.key)
					.. " to "
					.. tostring(t.value)
					.. " in frozen object "
					.. tostring(t.obj)
			else
				error("unknown, please report this is a bug.")
			end
		end,
	}

	function FrozenObjectError:new(obj, k, v)
		local o = { kind = "Set", obj = obj, key = k, value = v }
		setmetatable(o, self)
		self.__index = self
		return o
	end

	-- Freeze table.
	local freeze = nil
	freeze = function(t, opts)
		if rawtype(t) ~= "table" and rawtype(t) ~= "userdata" then return t end

		opts = opts or {}
		opts.shallow = opts.shallow or false
		opts.metatable = opts.metatable or false
		opts.replace = opts.replace or {}

		local t_mt = __rawgetmetatable(t)

		-- Return table if it is already frozen.
		if rawtype(t_mt) == "table" and t_mt.__frozen then return t end

		-- If this is a self referential table, returns already frozen table to
		-- prevent infinite loop.
		if opts.replace[t] then return opts.replace[t] end

		-- Create proxy table.
		local proxy = table.new(0, 0)
		opts.replace[t] = proxy

		-- Create proxy metatable.
		local proxy_mt = {
			__frozen = true,
			__index = t,
			__newindex = function(_, k, v)
				error(FrozenObjectError:new(t, k, v))
			end,
			__ipairs = function()
				return ipairs(t)
			end,
			__pairs = function()
				return pairs(t)
			end,
			-- fallback to false instead of nil otherwise,
			-- proxy_mt would be returned.
			__metatable = t_mt or false,
		}
		-- Set metatable of proxy metatable to fallback on table's metatable.
		-- This way we don't have to forward __tostring and other metamethod.
		setmetatable(proxy_mt, { __index = t_mt })

		-- Deep freeze.
		if not opts.shallow then
			proxy_mt.__index = function(_, k)
				return freeze(t[k], opts)
			end
		end

		-- Freeze metatable.
		if rawtype(t_mt) == "table" and opts.metatable then
			proxy_mt.__metatable = freeze(t_mt, opts)
		end

		-- Set proxy metatable.
		setmetatable(proxy, proxy_mt)

		return proxy
	end

	return freeze
end

local function breakpoint_impl()
	local debug = require("debug")
	local table = require("table")
	local term = require("term")
	local path = require("path")
	local os = require("os")

	local eval_incomplete = {}

	local eval = nil
	-- Eval Lua code read from the REPL in the given environment.
	eval = function(code, env)
		if code:has_prefix("local ") then
			-- REPL doesn't support local variables.
			code = code:slice(#"local " + 1)
		elseif not code:has_prefix("return ") and not code:contains("=") then
			-- Try to prefix "return " so loaded code returns its result.
			local ok, v = eval("return " .. code, env)
			if ok then
				return true, v
			elseif v == eval_incomplete then
				return false, eval_incomplete
			end
		end

		-- Load Lua code.
		local f, err = load(code, "repl", "t", env)

		-- Failed to load Lua code.
		if not f then
			if err:contains("<eof>") then
				-- Line is incomplete.
				return false, eval_incomplete
			end
			return false, err
		end

		return pcall(f)
	end

	function __repl(env)
		print("exit using ctrl+d, ctrl+c or close()")
		local multiline = {}
		local interrupted = false
		local closed = {}

		-- Add close function to stop REPL.
		local eval_env = setmetatable({
			close = function()
				error(closed)
			end,
		}, {
			__index = env,
			__newindex = env,
		})

		-- Create readline editor.
		local ed = term.ReadLine.new()
		local history_path =
			path.join(os.data_local_dir() or os.temp_dir(), "allelua", "history")
		os.create_dir_all(path.parent(history_path))
		pcall(ed.load_history, ed, history_path)

		while true do
			local ok, line =
				pcall(ed.read_line, ed, #multiline == 0 and "> " or ">> ")
			if not ok then
				local err = line
				if err.kind == "eof" then break end
				if err.kind == "interrupted" then
					if interrupted then break end
					print("press ctrl+c again to exit")
					interrupted = true
				end
			else
				table.push(multiline, line)

				-- selene: allow(shadowing)
				local ok, value = eval(table.concat(multiline, "\n"), eval_env)
				if not ok then
					local err = value
					if err == closed then
						break
					elseif err ~= eval_incomplete then
						print("Error: ", err)
						multiline = {}
					end
					interrupted = false
				else
					multiline = {}
					interrupted = false
					print(value)
				end
			end
		end

		-- Save history.
		ed:save_history(history_path)
	end

	return function()
		local info = debug.getinfo(2, "fSul")
		if not info then return end
		print(
			traceback(
				"breakpoint reached at "
					.. info.short_src
					.. ":"
					.. tostring(info.currentline),
				2
			)
		)

		local variables = {}
		for i = 2, 1024 do
			if i > 2 and not debug.getinfo(i, "f") then break end

			local j = 1
			while true do
				local name, value = debug.getlocal(i, j)
				if not name then break end
				if not variables[name] then variables[name] = value end
				j = j + 1
			end

			j = 1
		end
		for i = 1, 1024 do
			local name, value = debug.getupvalue(info.func, i)
			if not name then break end
			-- Upvalue is not shadowed by local variable.
			if not variables[name] then variables[name] = value end
		end

		local env = getfenv()
		local repl_env = setmetatable({}, {
			__index = function(_, k)
				return variables[k] or env[k]
			end,
			__newindex = function(_, k, v)
				for i = 4, 1024 do
					if not debug.getinfo(i, "f") then break end

					for j = 1, 512 do
						-- Try local first.
						local name, value = debug.getlocal(i, j)
						if not name then break end
						if name == k and value ~= v then
							variables[k] = v
							debug.setlocal(i, j, v)
						end

						-- Upvalues are tied to function and not stack level.
						-- There is no need to run this if it failed at first level (i == 4).
						if i == 4 then
							-- Then try upvalue
							name, value = debug.getupvalue(info.func, j)
							if not name then break end
							if name == k and value ~= v then
								variables[k] = v
								debug.setupvalue(info.func, j, v)
							end
						end
					end
				end

				return nil
			end,
			__pairs = function(_)
				return pairs(variables)
			end,
		})

		_G.debug = debug
		__repl(repl_env)
		_G.debug = nil

		debug.variables = nil
	end
end

function import_impl()
	local debug = require("debug")
	local path = require("path")

	return function(pkgpath)
		if pkgpath:has_prefix("./") then
			local caller = debug.getinfo(2).short_src
			pkgpath = path.join(path.parent(caller), pkgpath)
		end
		local pkgname = path.file_name(pkgpath)

		local pkg = require(pkgpath)
		_G[pkgname] = pkg
		return pkg
	end
end

return function(M)
	M.pcall = pcall_impl()
	M.xpcall = xpcall_impl()
	M.tostring = tostring_impl()
	M.clone = clone_impl()
	M.switch = switch_impl
	M.freeze = freeze_impl()
	M.breakpoint = breakpoint_impl()
	M.import = import_impl()
end
