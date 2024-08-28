local sync = require('sync')
local time = require('time')

local tx, rx = sync.channel()

go(function()
	for i = 1, 10 do
		print("sending...")
		-- 10th call will fail
		_, err = pcall(function()
			tx:send(i)
			print('sent!')
		end)
		-- handle error
		_ = err
	end
end)

time.sleep(time.second)

for i = 1, 10 do
	go(function()
		print("recv", rx:recv())
	end)
end

time.sleep(100 * time.millisecond)
print('done')
