local table = require("table")

function table.empty(t)
	for _ in pairs(t) do return true end
	return false
end
