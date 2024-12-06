return function(is_empty)
	local table = require("table")
	local math = require("math")
	local M = table

	-- LuaJIT extensions.
	M.new = require("table.new")
	M.clear = require("table.clear")

	M.for_each = M.foreach
	M.foreach = nil
	M.ifor_each = M.foreachi
	M.foreachi = nil

	local sort = M.sort
	M.sort = function(t, comp)
		sort(t, comp)
		return t
	end

	M.is_empty = is_empty

	M.reverse = function(t)
		local len = M.getn(t)
		for i = 1, math.floor(len / 2) do
			local a = t[i]
			local b = t[len - i + 1]
			t[i] = b
			t[len - i + 1] = a
		end
		return t
	end

	M.binary_search = function(t, elem)
		local left = 1
		local right = M.getn(t)

		while left <= right do
			local mid = left + math.round((right - left) / 2)

			if t[mid] == elem then
				return mid
			elseif t[mid] < elem then
				left = mid + 1
			else
				right = mid - 1
			end
		end

		return nil
	end

	M.push = function(t, ...)
		local args = { ... }
		for _, v in ipairs(args) do
			M.insert(t, v)
		end
		return M.getn(t)
	end

	M.pop = function(t)
		return M.remove(t)
	end

	M.unshift = function(t, ...)
		local args = { ... }
		for _, v in ipairs(args) do
			M.insert(t, 1, v)
		end
		return M.getn(t)
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

	M.slice = function(t, i, j)
		local len = M.getn(t)
		i = i or 1
		j = j or len

		if i < 0 then i = len + 1 + i end
		if j < 0 then j = len + 1 + j end

		if i <= 0 then i = 1 end
		if j > len then j = len end

		if j < i then return {} end
		local slice = M.new(j + 1 - i, 0)
		for index = i, j do
			table.push(slice, t[index])
		end

		return slice
	end

	M.map = function(t, map_fn)
		local result = {}
		for k, v in pairs(t) do
			local new_k, new_v = map_fn(k, v)
			if new_v == nil then
				new_v = new_k
				new_k = k
			end
			result[new_k] = new_v
		end
		setmetatable(result, getmetatable(t))
		return result
	end

	M.map_keys = function(t, map_fn)
		return M.map(t, function(k, v)
			k = map_fn(k)
			return k, v
		end)
	end
	M.map_values = function(t, map_fn)
		return M.map(t, function(k, v)
			v = map_fn(v)
			return k, v
		end)
	end

	M.deep_eq = function(a, b, seen)
		if a == b then return true end

		-- If either value is not a table, they're not equal (since a ~= b)
		if rawtype(a) ~= "table" or rawtype(b) ~= "table" then return false end

		-- We can't use M.is_empty for fast checks has table may have __index
		-- metamethod.

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
			if not M.deep_eq(v, b[k], seen) then return false end
		end

		-- Check if 'b' has any keys that 'a' doesn't have
		for k in pairs(b) do
			if a[k] == nil then return false end
		end

		return true
	end
end
