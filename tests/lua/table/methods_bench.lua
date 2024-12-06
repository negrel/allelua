local t = require("test")
local table = require("table")

t.bench("insert 1000 elements in table", function(b)
	for _ = 1, b.n do
		local tab = {}
		for i = 1, 1000 do
			table.insert(tab, i)
		end
	end
end)

t.bench("push 1000 elements in table", function(b)
	for _ = 1, b.n do
		local tab = {}
		for i = 1, 1000 do
			table.push(tab, i)
		end
	end
end)
