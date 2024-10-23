local sync = require("sync")
local t = require("test")
local time = require("time")

t.bench("send on unbuffered channel", function(b)
	local tx, rx = sync.channel()

	coroutine.nursery(function(go)
		go(function()
			for i = 1, b.n do
				tx:send(i)
			end
		end)

		for _ = 1, b.n do
			rx:recv()
		end
	end)
end)

t.bench("send on a channel with a 1000 buffer", function(b)
	local tx, rx = sync.channel(1000)

	coroutine.nursery(function(go)
		go(function()
			for i = 1, b.n do
				tx:send(i)
			end
		end)

		for _ = 1, b.n do
			rx:recv()
		end
	end)
end)
