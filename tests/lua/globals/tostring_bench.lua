local t = require("test")

t.bench("allelua tostring", function(b)
	local tostring = tostring

	for _ = 0, b.n do
		tostring("string")
		tostring(3.14)
		tostring(true)
		tostring(math.huge)
		tostring { 1, 2, 3 }
	end
end)

t.bench("luajit tostring", function(b)
	local tostring = rawtostring

	for _ = 0, b.n do
		tostring("string")
		tostring(3.14)
		tostring(true)
		tostring(math.huge)
		tostring { 1, 2, 3 }
	end
end)
