local table = table

local frozen_tables = setmetatable({}, { __mode = "k" })

local freeze_mt = {
	__metatable = function(t)
		return table.freeze(getmetatable(t))
	end,
	__index = function(t, k)
		local v = frozen_tables[t][k]
		if type(v) == "table" then
			return table.freeze(v)
		end
		return v
	end,
	__newindex = function(t, k, v) error("frozen table") end
}

--- Return a new frozen (read-only) table that wraps given table.
function table.freeze(t)
	assert(type(t) == "table", "freeze require a table argument")
	-- Already frozen.
	if frozen_tables[t] then return t end

	local frozen = {}
	frozen_tables[frozen] = t
	return setmetatable(frozen, freeze_mt)
end

--- Return whether table is empty or not.
function table.is_empty(t)
	for _ in pairs(t) do
		return false
	end
	return true
end

--- Collect keys and values from given iterator into a table.
function table.collect_kv(...)
	local t = {}
	for k, v in ... do t[k] = v end
	return t
end

--- Collect values from given iterator into a table.
function table.collect(...)
	local t = {}
	for v in ... do table.insert(t, v) end
	return t
end
