return function(is_empty)
	local table = require("table")
	local M = table

	-- LuaJIT extensions.
	M.new = require("table.new")
	M.clear = require("table.clear")

	-- Frozen table error.
	local FrozenTableError = {
		__type = "FrozenTableError",
		__tostring = function(t)
			if t.kind == "Set" then
				return "cannot set "
					.. tostring(t.key)
					.. " to "
					.. tostring(t.value)
					.. " in frozen table "
					.. tostring(t.table)
			else
				error("unknown, please report this is a bug.")
			end
		end,
	}

	function FrozenTableError:new(table, k, v)
		local o = { kind = "Set", table = table, key = k, value = v }
		setmetatable(o, self)
		self.__index = self
		return o
	end

	-- Freeze table.
	M.freeze = function(t, opts)
		assert(
			rawtype(t) == "table" or rawtype(t) == "userdata",
			"invalid input: " .. tostring(t) .. " is not table like"
		)

		opts = opts or {}
		opts.deep = opts.deep or false
		opts.metatable = opts.metatable or false
		opts.replace = opts.replace or {}

		local t_mt = __rawgetmetatable(t)

		-- Return table if it is already frozen.
		if rawtype(t_mt) == "table" and t_mt.__frozen then return t end

		-- If this is a self referential table, returns already frozen table to
		-- prevent infinite loop.
		if opts.replace[t] then return opts.replace[t] end

		-- Create proxy table.
		local proxy = M.new(0, 0)
		opts.replace[t] = proxy

		-- Create proxy metatable.
		local proxy_mt = {
			__frozen = true,
			__index = t,
			__newindex = function(_, k)
				error(FrozenTableError:new(t, k))
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
		-- This way we don't have to forward __tostring and other metamethod unless
		-- they're implemented.
		setmetatable(proxy_mt, { __index = t_mt })

		-- Freeze metatable.
		if rawtype(t_mt) == "table" and opts.metatable then
			proxy_mt.__metatable = M.freeze(t_mt, opts)
		end

		-- Set proxy metatable.
		setmetatable(proxy, proxy_mt)

		return proxy
	end

	M.is_frozen = function(t)
		local t_mt = __rawgetmetatable(t)
		return rawtype(t_mt) == "table" and t_mt.__frozen
	end

	M.is_empty = is_empty

	M.push = function(t, ...)
		local args = { ... }
		for _, v in ipairs(args) do
			M.insert(t, v)
		end
		return #t
	end
	M.pop = function(t)
		return M.remove(t)
	end
	M.unshift = function(t, ...)
		local args = { ... }
		for _, v in ipairs(args) do
			M.insert(t, 1, v)
		end
		return #t
	end
	M.shift = function(t)
		return M.remove(t, 1)
	end

	M.indexof = function(t, elem, start)
		start = start or 1
		local i = start
		while true do
			if t[i] == elem then return i end
			if t[i] == nil then break end
			i = i + 1
		end
		return -1
	end

	M.map = function(t, map_fn)
		local result = {}
		for k, v in pairs(t) do
			local new_k, new_v = map_fn(k, v)
			if new_v == nil then
				new_v = new_k
				new_k = k
			end
			t[new_k] = new_v
		end
		setmetatable(result, getmetatable(t))
		return result
	end

	M.deep_eq = function(a, b, seen)
		if a == b then return true end

		-- If either value is not a table, they're not equal (since a ~= b)
		if type(a) ~= "table" or type(b) ~= "table" then return false end

		-- We can't use M.is_empty for fast checks has table may have __index
		-- metamethod.
		-- if M.is_empty(a) ~= M.is_empty(b) then return false end

		-- Check for cycles
		seen = seen or {}
		if seen[a] and seen[a][b] then
			return true -- We've seen this pair before, consider them equal to avoid infinite recursion
		end
		seen[a] = seen[a] or {}
		seen[a][b] = true
		seen[b] = seen[b] or {}
		seen[b][a] = true

		-- Check if all keys in 'a' exist in 'b' and have the same values
		for k, v in pairs(a) do
			if not M.deep_eq(v, b[k], seen) then
				print("a has keys different in b", k)
				return false
			end
		end

		-- Check if 'b' has any keys that 'a' doesn't have
		for k in pairs(b) do
			if a[k] == nil then
				print("b has keys not in a", k)
				return false
			end
		end

		return true
	end

	M.collect_map = function(map_fn)
		return function(iterator, state, initial_value)
			local result = {}
			for k, v in iterator, state, initial_value do
				table.insert(result, map_fn(k, v))
			end

			return result
		end
	end

	M.collect = M.collect_map(M.pack)
	M.collect_entries = M.collect_map(function(k, v)
		return { k, v }
	end)
	M.collect_keys = M.collect_map(function(k, _v)
		return k
	end)
	M.collect_values = M.collect_map(function(_k, v)
		return v
	end)

	M.keys = function(t)
		return M.collect_keys(pairs(t))
	end
	M.values = function(t)
		return M.collect_values(pairs(t))
	end
	M.ivalues = function(t)
		return M.collect_values(ipairs(t))
	end
	M.entries = function(t)
		return M.collect_entries(pairs(t))
	end
	M.from_entries = function(entries)
		local result = {}
		for _, entry in ipairs(entries) do
			local k, v = table.unpack(entry)
			result[k] = v
		end

		return result
	end
end
