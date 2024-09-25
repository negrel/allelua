local sync = require("sync")
local time = require("time")
local t = require("test")

t.bench("1_000_000 send on unbuffered channel", function(b)
	local tx, rx = sync.channel()

	go(function()
		for i = 1, b.n do
			tx:send(i)
		end
	end)

	for _ = 1, b.n do
		rx:recv()
	end
end)

t.bench("1_000_000 send on a channel with a 1000 buffer", function(b)
	local tx, rx = sync.channel(1000)

	go(function()
		for i = 1, b.n do
			tx:send(i)
		end
	end)

	for _ = 1, b.n do
		rx:recv()
	end
end)
