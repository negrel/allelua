local sync = require("sync")
local time = require("time")

-- Change channel buf size and observes ouput.
local size = 0
local tx, rx = sync.channel(size)

go(function()
	for i = 1, 10 do
		tx:send(i)
		print("value sent!")
	end
end)

for v in rx:iter() do
	print("recv", v)
end
