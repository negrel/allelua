local sync = require('sync')
local time = require('time')
local tx, rx = sync.mpsc(1)

go(function()
	for i = 1, 10 do
		print("sending...")
		-- 10th call will fail
		_, err = pcall(function()
			tx:send(i)
		end)
		-- handle error
	end
end)

time.sleep(time.second)

go(function()
	for i = 1, 9 do
		print("recv", rx:recv())
	end
	-- Close channel after 9 messages.
	rx:close()
end)

time.sleep(100 * time.millisecond)
print('done')
