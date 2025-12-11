local table = table
table.pairs = pairs
table.ipairs = ipairs
table.next = next
table.clear = require("table.clear")

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

