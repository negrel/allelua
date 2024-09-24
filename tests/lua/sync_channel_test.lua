local sync = require("sync")
local time = require("time")
local t = require("test")

t.test("1_000_000 send on unbuffered channel", function()
	local n = 1000000
	local tx, rx = sync.channel()

	go(function()
		for i = 1, n do
			tx:send(i)
		end
	end)

	for i = 1, n do
		rx:recv()
	end
end, { timeout = 10 * time.second})

t.test("1_000_000 send on a channel with a 1000 buffer", function()
	local n = 1000000
	local tx, rx = sync.channel(1000)

	go(function()
		for i = 1, n do
			tx:send(i)
		end
	end)

	for i = 1, n do
		rx:recv()
	end
end, { timeout = 10 * time.second})
