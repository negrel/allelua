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

local function tostring_impl()
	local string = require("string")
	local table = require("table")

	local rawtostring = tostring

	local tostring = nil

	local function tostring_table(value, opts)
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

		-- empty table ?
		if #items == 0 then return "{}" end

		return "{ " .. table.concat(items, ", ") .. close
	end

	-- selene: allow(shadowing)
	local function tostring_impl(value, opts)
		-- Call metamethod if any.
		local v_mt = getmetatable(value)
		if type(v_mt) == "table" and v_mt.__tostring ~= nil then
			return v_mt.__tostring(value, opts)
		end

		-- Custom default tostring for table.
		if type(value) == "table" then return tostring_table(value, opts) end

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

		if type(v) == "function" or v == nil then return rawtostring(v) end

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

local function clone_impl(clone_not_impl_err)
	local clone = function(v, opts)
		if rawtype(v) == "table" then
			local meta = getmetatable(v)
			if meta then
				if rawtype(meta.__clone) == "function" then
					return meta.__clone(v, opts)
				else
					return meta.__clone
				end
			end
			return clone_not_impl_err
		elseif rawtype(v) == "userdata" then
			if v.__clone then
				if rawtype(v.__clone) == "function" then
					return v.__clone(v, opts)
				else
					return v.__clone
				end
			end
			return clone_not_impl_err
		end

		return v
	end

	return function(v, opts)
		opts = opts or {}
		opts.deep = opts.deep or false
		opts.replace = opts.replace or {}
		local replace = opts.replace

		if replace[v] then return replace[v] end

		return clone(v, opts)
	end
end

local function switch_impl(v, cases, default)
	local case = cases[v]
	if case then
		case()
	else
		if default then default() end
	end
end

return function(M, clone_not_impl_err)
	M.pcall = pcall_impl()
	M.tostring = tostring_impl()
	M.clone = clone_impl(clone_not_impl_err)
	M.switch = switch_impl
end
