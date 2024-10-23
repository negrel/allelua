local coroutine = require("coroutine")
local t = require("test")

t.bench("spawning 1_000 coroutines in nursery", function(b)
	for _ = 0, b.n do
		coroutine.nursery(function(go)
			for _ = 0, 1000 do
				go(function() end)
			end
		end)
	end
end)
