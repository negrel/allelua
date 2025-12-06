local table = table
table.pairs = pairs
table.ipairs = ipairs
table.next = next
table.clear = require("table.clear")

local frozen2table = setmetatable({}, { __mode = "kv" })
local table2frozen = setmetatable({}, { __mode = "kv" })

local freeze_mt = {
	__metatable = function(t)
		return table.freeze(getmetatable(frozen2table[t]))
	end,
	__index = function(t, k)
		local v = frozen2table[t][k]
		if type(v) == "table" then
			return table.freeze(v)
		end
		return v
	end,
	__pairs = function(t, k) return pairs(frozen2table[t]) end,
	__ipairs = function(t, k) return ipairs(frozen2table[t]) end,
	__newindex = function(t, k, v) error("frozen table") end
}

--- Return a new frozen (read-only) table that wraps given table.
function table.freeze(t)
	assert(type(t) == "table", "freeze require a table argument")
	-- Already frozen.
	if frozen2table[t] then return t end
	-- Table already has a frozen instance.
	if table2frozen[t] then return table2frozen[t] end

	local frozen = setmetatable({}, freeze_mt)
	frozen2table[frozen] = t
	table2frozen[t] = frozen
	return frozen
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
