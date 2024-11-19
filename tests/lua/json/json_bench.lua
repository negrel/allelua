local json = require("json")
local t = require("test")

t.bench(
	"json encoding table with 3 levels of depths and 3 keys at each level",
	function(b)
		local lvl1 =
			{ number = 1, boolean = true, string = "a nice string at level 1" }
		local lvl2 =
			{ number = 2, boolean = true, string = "a nice string at level 2" }
		local lvl3 =
			{ number = 3, boolean = true, string = "a nice string at level 3" }

		lvl1.next = lvl2
		lvl2.next = lvl3

		for _ = 0, b.n do
			local _str = json.encode(lvl1)
		end
	end
)

t.bench(
	"json pretty encoding table with 3 levels of depths and 3 keys at each level",
	function(b)
		local lvl1 =
			{ number = 1, boolean = true, string = "a nice string at level 1" }
		local lvl2 =
			{ number = 2, boolean = true, string = "a nice string at level 2" }
		local lvl3 =
			{ number = 3, boolean = true, string = "a nice string at level 3" }

		lvl1.next = lvl2
		lvl2.next = lvl3

		for _ = 0, b.n do
			local _str = json.encode(lvl1, { pretty = true })
		end
	end
)
