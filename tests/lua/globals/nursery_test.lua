local coroutine = require("coroutine")
local sync = require("sync")
local t = require("test")
local time = require("time")

t.test("goroutines runs concurrently", function()
	local is_seq = true

	coroutine.nursery(function(go)
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

		for i = 1, 1000 do
			local v, ok = rx:recv()
			assert(v ~= nil and ok)
			is_seq = is_seq and v == i
		end
	end)

	assert(not is_seq, "goroutines execution is sequential")
end, { timeout = 10 * time.second })

t.test("abort goroutine", function()
	local goroutine_complete = false

	coroutine.nursery(function(go)
		local abort = go(function()
			time.sleep(5 * time.millisecond)
			goroutine_complete = true
		end)

		time.sleep(time.millisecond)
		abort()
		time.sleep(10 * time.millisecond)
	end)

	assert(not goroutine_complete, "goroutine abort failed")
end)

t.test("go with function and its args as parameters", function()
	local value = nil
	local func = function(arg)
		value = arg
	end

	coroutine.nursery(function(go)
		go(func, t)
	end)

	assert(value ~= nil)
	assert(value == t)
end)
