local sync = require("sync")
local time = require("time")
local t = require("test")

t.test("goroutines runs concurrently", function()
	local tx, rx = sync.channel()
	local wg = sync.WaitGroup.new()
	wg:add(1000)
	for i = 1, 1000 do
		go(function()
			time.sleep(1 * time.microsecond)
			tx:send(i)
			wg:done()
		end)

		go(function()
			wg:wait()
			tx:close()
		end)
	end

	local is_seq = true
	for i = 1, 1000 do
		local v, ok = rx:recv()
		assert(v ~= nil and ok)
		is_seq = is_seq and v == i
	end


	assert(not is_seq, "goroutines execution is sequential")
end, { timeout = 10 * time.second })

t.test("abort goroutine", function()
	local goroutine_complete = false
	local abort = go(function()
		time.sleep(5 * time.millisecond)
		goroutine_complete = true
	end)

	time.sleep(time.millisecond)
	abort()
	time.sleep(10 * time.millisecond)
	assert(not goroutine_complete, "goroutine abort failed")
end)
