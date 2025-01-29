local coroutine = require("coroutine")
local sync = require("sync")
local t = require("test")
local time = require("time")

t.test("simple mutex usage", function()
	local mu = sync.Mutex.new { counter = 1 }
	local v = mu:lock()
	assert(v.counter == 1)
	mu:unlock()
end)

t.test("double unlock mutex", function()
	local mu = sync.Mutex.new { counter = 1 }
	mu:unlock()
	mu:unlock()
end)

t.test("concurrent mutex access creates contention", function()
	local mu = sync.Mutex.new { counter = 1 }

	local wait_dur = nil
	coroutine.nursery(function(go)
		go(function()
			local v = mu:lock()
			time.sleep(100 * time.millisecond)
			v.counter = 2
			mu:unlock()
		end)

		go(function()
			local now = time.Instant.now()
			local v = mu:lock()
			assert(v.counter == 2)
			v.counter = 3
			mu:unlock()
			wait_dur = now:elapsed()
		end)

		go(function()
			local v = mu:lock()
			assert(v.counter == 3)
			mu:unlock()
		end)
	end)

	assert(wait_dur > 100 * time.millisecond)
	local v = mu:lock()
	assert(v.counter == 3)
	mu:unlock()
end)
