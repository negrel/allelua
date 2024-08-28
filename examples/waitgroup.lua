local time = require('time')
local sync = require('sync')

local wg = sync.waitgroup()
wg:add(2)

function work(n, secs)
	for i = 0, secs - 1 do
		print('goroutine', n, 'is working...', (secs - i) * time.second)
		time.sleep(time.second)
	end
	print('goroutine', n, 'work is done!')
end

go(function()
	work(1, 3)
	wg:done()
end)

go(function()
	work(2, 1)
	wg:done()
end)

print("waiting for group...")
wg:wait()
print("all work is done!")
