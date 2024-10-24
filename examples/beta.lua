local time = require("time")

local go2 = nil

coroutine.nursery(function(go)
	go2 = go
	local now = time.Instant:now()
	for i = 1, 3 do
		go(function()
			time.sleep(1 * time.second)
			print("goroutine", i, "done in", now:elapsed())
		end)
	end
end)
print("done")
go2(function()
	print("go2")
end)
