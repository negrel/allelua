local time = require("time")
local sync = require("sync")

local tx1, rx1 = sync.channel()
local tx2, rx2 = sync.channel()

go(function()
	tx1:send("tx1")
end)

go(function()
	tx2:send("tx2")
end)

local timeout, abort_timeout = time.after(2 * time.second)

local done = false
while not done do
	select {
		[rx1] = function(...)
			abort_timeout()
			print("rx1", ...)
		end,
		[rx2] = function(...)
			abort_timeout()
			print("rx2", ...)
		end,
		[timeout] = function()
			print("timeout after 2s")
			done = true
		end,
		default = function()
			print("default")
		end
	}
end
