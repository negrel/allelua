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

--- Returns a new frozen (read-only) table that wraps given table.
function table.freeze(t)
	assert(type(t) == "table", "freeze require a table argument")
	-- Already frozen.
	if frozen_tables[t] then return t end

	local frozen = {}
	frozen_tables[frozen] = t
	return setmetatable(frozen, freeze_mt)
end

