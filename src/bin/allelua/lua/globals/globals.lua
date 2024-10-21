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
	local rawtostring = tostring

	local tostring = nil

	local function tostring_pairs(value, opts)
		local space = opts.space <= 0 and ""
			or "\n" .. string.rep(" ", opts.space * opts.depth)
		local close = opts.space <= 0 and " }"
			or "\n" .. string.rep(" ", opts.space * (opts.depth - 1)) .. "}"

		local inner_opts = {
			space = opts.space,
			depth = opts.depth + 1,
			__stringified = opts.__stringified,
		}

		local items = {}
		for k, v in pairs(value) do
			local kv = { space }
			if rawtype(k) == "string" or rawtype(k) == "number" then
				table.push(kv, k)
			else
				table.push(kv, "[", tostring(k, inner_opts), "]")
			end
			table.push(kv, " = ")

			if rawtype(v) == "string" then
				table.push(kv, string.format("%q", v))
			else
				table.push(kv, tostring(v, inner_opts))
			end

			table.push(items, table.concat(kv))
		end

		-- empty table ?
		if #items == 0 then return "{}" end

		return "{ " .. table.concat(items, ", ") .. close
	end

	-- selene: allow(shadowing)
	local function tostring_impl(value, opts)
		-- Call metamethod if any.
		local v_mt = rawgetmetatable(value)

		if is_table_like(v_mt) then
			if rawtype(v_mt.__tostring) == "function" then
				return v_mt.__tostring(value, opts)
			end

			-- Custom default tostring for __pairs.
			if rawtype(v_mt.__pairs) == "function" then
				return tostring_pairs(value, opts)
			end
		end

		-- Custom default tostring for table.
		if rawtype(value) == "table" then return tostring_pairs(value, opts) end

		return rawtostring(value)
	end

	-- A custom to string function that pretty format table and support
	-- recursive values.
	tostring = function(v, opts)
		opts = opts or {}
		opts.__stringified = opts.__stringified or {}
		local stringified = opts.__stringified

		opts.space = opts.space or 2
		opts.depth = opts.depth or 1

		if rawtype(v) == "function" or v == nil then return rawtostring(v) end

		if stringified[v] then
			stringified[v] = stringified[v] + 1
			return rawtostring(v)
		end
		stringified[v] = 1

		local result = tostring_impl(v, opts)

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
			elseif not opts.metatable.shallow then
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

return function(M)
	M.pcall = pcall_impl()
	M.xpcall = xpcall_impl()
	M.tostring = tostring_impl()
	M.clone = clone_impl()
	M.switch = switch_impl
	M.freeze = freeze_impl()
end
